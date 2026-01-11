use std::sync::Arc;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, Message, MessageRole, MessageStatus, TextBlock, ToolStatus};
use crate::agent::utils::tokenizer::count_message_param_tokens;
use crate::storage::DatabaseManager;

use super::compactor::SessionCompactor;
use super::config::CompactionConfig;
use super::loader::SessionMessageLoader;
use super::pruner::Pruner;
use super::result::{CompactionPhase, CompactionResult};

#[derive(Debug, Clone, Copy)]
pub enum CompactionTrigger {
    Auto,
    Manual,
}

#[derive(Debug, Clone)]
pub struct PreparedCompaction {
    pub result: CompactionResult,
    pub summary_job: Option<SummaryJob>,
}

#[derive(Debug, Clone)]
pub struct SummaryJob {
    pub summary_message: Message,
    pub started_at: chrono::DateTime<Utc>,
    pub conversation_text: String,
}

#[derive(Debug, Clone)]
pub struct SummaryCompletion {
    pub message_id: i64,
    pub status: MessageStatus,
    pub finished_at: chrono::DateTime<Utc>,
    pub duration_ms: i64,
}

#[derive(Clone)]
pub struct CompactionService {
    repositories: Arc<DatabaseManager>,
    persistence: Arc<AgentPersistence>,
    config: CompactionConfig,
}

impl CompactionService {
    pub fn new(
        repositories: Arc<DatabaseManager>,
        persistence: Arc<AgentPersistence>,
        config: CompactionConfig,
    ) -> Self {
        Self {
            repositories,
            persistence,
            config,
        }
    }

    pub async fn prepare_compaction(
        &self,
        session_id: i64,
        context_window: u32,
        trigger: CompactionTrigger,
    ) -> AgentResult<PreparedCompaction> {
        if !self.config.enabled || context_window == 0 {
            return Ok(PreparedCompaction {
                result: skipped(),
                summary_job: None,
            });
        }

        let loader = SessionMessageLoader::new(Arc::clone(&self.persistence));
        let before_messages = loader.load_for_llm(session_id).await?;
        let tokens_before = estimate_tokens(&before_messages);

        if matches!(trigger, CompactionTrigger::Auto)
            && tokens_before < (context_window as f32 * self.config.prune_threshold) as u32
        {
            return Ok(PreparedCompaction {
                result: skipped_with(tokens_before),
                summary_job: None,
            });
        }

        let pruner = Pruner::new(Arc::clone(&self.persistence), self.config.clone());
        let tools_compacted = pruner.prune_session(session_id).await?;

        let after_prune_messages = loader.load_for_llm(session_id).await?;
        let tokens_after_prune = estimate_tokens(&after_prune_messages);

        let should_compact = match trigger {
            CompactionTrigger::Manual => true,
            CompactionTrigger::Auto => {
                tokens_after_prune >= (context_window as f32 * self.config.compact_threshold) as u32
                    || self.message_count_with_breakpoint(session_id).await?
                        > self.config.max_messages_before_compact
            }
        };

        if !should_compact
            || (matches!(trigger, CompactionTrigger::Auto) && !self.config.auto_compact)
        {
            return Ok(PreparedCompaction {
                result: CompactionResult {
                    phase: CompactionPhase::Pruned,
                    tokens_before,
                    tokens_after: tokens_after_prune,
                    tokens_saved: tokens_before.saturating_sub(tokens_after_prune),
                    tools_compacted,
                    summary_created: false,
                    summary_message_id: None,
                },
                summary_job: None,
            });
        }

        let messages = self
            .persistence
            .messages()
            .list_by_session_with_breakpoint(session_id)
            .await?;
        let Some(scope) = SummaryScope::new(&messages, self.config.keep_recent_messages) else {
            return Ok(PreparedCompaction {
                result: CompactionResult {
                    phase: CompactionPhase::Pruned,
                    tokens_before,
                    tokens_after: tokens_after_prune,
                    tokens_saved: tokens_before.saturating_sub(tokens_after_prune),
                    tools_compacted,
                    summary_created: false,
                    summary_message_id: None,
                },
                summary_job: None,
            });
        };

        let started_at = Utc::now();
        let mut summary_message = self
            .persistence
            .messages()
            .create(
                session_id,
                MessageRole::Assistant,
                MessageStatus::Streaming,
                Vec::new(),
                true,
            )
            .await?;

        sqlx::query("UPDATE messages SET created_at = ? WHERE id = ?")
            .bind(scope.boundary_ts)
            .bind(summary_message.id)
            .execute(self.repositories.pool())
            .await?;
        summary_message.created_at = chrono::Utc
            .timestamp_opt(scope.boundary_ts, 0)
            .single()
            .unwrap_or_else(Utc::now);

        let conversation_text = self.render_messages_for_summary(scope.messages).await?;
        let summary_id = summary_message.id;

        Ok(PreparedCompaction {
            result: CompactionResult {
                phase: CompactionPhase::Started,
                tokens_before,
                tokens_after: tokens_after_prune,
                tokens_saved: tokens_before.saturating_sub(tokens_after_prune),
                tools_compacted,
                summary_created: true,
                summary_message_id: Some(summary_id),
            },
            summary_job: Some(SummaryJob {
                summary_message,
                started_at,
                conversation_text,
            }),
        })
    }

    pub async fn complete_summary_job(
        &self,
        mut job: SummaryJob,
        model_id: &str,
    ) -> AgentResult<SummaryCompletion> {
        let summary_text = if job.conversation_text.trim().is_empty() {
            String::new()
        } else {
            let compactor =
                SessionCompactor::new(Arc::clone(&self.repositories), self.config.clone());
            compactor
                .generate_summary(model_id, &job.conversation_text)
                .await?
        };

        let finished_at = Utc::now();
        let duration_ms = finished_at
            .signed_duration_since(job.started_at)
            .num_milliseconds()
            .max(0) as i64;

        job.summary_message.status = MessageStatus::Completed;
        job.summary_message.finished_at = Some(finished_at);
        job.summary_message.duration_ms = Some(duration_ms);
        job.summary_message.blocks = vec![Block::Text(TextBlock {
            id: Uuid::new_v4().to_string(),
            content: summary_text,
            is_streaming: false,
        })];

        self.persistence
            .messages()
            .update(&job.summary_message)
            .await?;

        Ok(SummaryCompletion {
            message_id: job.summary_message.id,
            status: job.summary_message.status.clone(),
            finished_at,
            duration_ms,
        })
    }

    pub async fn clear_compaction_for_session(&self, session_id: i64) -> AgentResult<()> {
        self.persistence
            .tool_outputs()
            .clear_compaction_marks_for_session(session_id)
            .await?;

        let messages = self
            .persistence
            .messages()
            .list_by_session(session_id)
            .await?;
        for mut message in messages {
            let was_summary = message.is_summary;
            if was_summary {
                message.is_summary = false;
            }

            let mut changed = false;
            for block in &mut message.blocks {
                if let Block::Tool(tool) = block {
                    if tool.compacted_at.is_some() {
                        tool.compacted_at = None;
                        changed = true;
                    }
                }
            }
            if changed || was_summary {
                self.persistence.messages().update(&message).await?;
            }
        }

        Ok(())
    }

    async fn message_count_with_breakpoint(&self, session_id: i64) -> AgentResult<usize> {
        Ok(self
            .persistence
            .messages()
            .list_by_session_with_breakpoint(session_id)
            .await?
            .len())
    }

    async fn render_messages_for_summary(&self, messages: &[Message]) -> AgentResult<String> {
        if messages.is_empty() {
            return Ok(String::new());
        }

        let message_ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
        let outputs = self
            .persistence
            .tool_outputs()
            .list_by_message_ids(&message_ids)
            .await?;

        let mut map = std::collections::HashMap::new();
        for (mid, bid, content, compacted_at) in outputs {
            map.insert((mid, bid), (content, compacted_at));
        }

        Ok(render_messages(messages, &map))
    }
}

struct SummaryScope<'a> {
    boundary_ts: i64,
    messages: &'a [Message],
}

impl<'a> SummaryScope<'a> {
    fn new(messages: &'a [Message], keep_recent_messages: usize) -> Option<Self> {
        if messages.len() <= keep_recent_messages {
            return None;
        }

        let split_at = messages.len().saturating_sub(keep_recent_messages);
        let boundary_ts = messages[split_at].created_at.timestamp().saturating_sub(1);

        Some(Self {
            boundary_ts,
            messages: &messages[..split_at],
        })
    }
}

fn estimate_tokens(messages: &[crate::llm::anthropic_types::MessageParam]) -> u32 {
    messages
        .iter()
        .map(|m| count_message_param_tokens(m) as u32)
        .fold(0u32, |acc, n| acc.saturating_add(n))
}

fn skipped() -> CompactionResult {
    CompactionResult {
        phase: CompactionPhase::Skipped,
        tokens_before: 0,
        tokens_after: 0,
        tokens_saved: 0,
        tools_compacted: 0,
        summary_created: false,
        summary_message_id: None,
    }
}

fn skipped_with(tokens: u32) -> CompactionResult {
    CompactionResult {
        phase: CompactionPhase::Skipped,
        tokens_before: tokens,
        tokens_after: tokens,
        tokens_saved: 0,
        tools_compacted: 0,
        summary_created: false,
        summary_message_id: None,
    }
}

fn render_messages(
    messages: &[Message],
    tool_outputs: &std::collections::HashMap<(i64, String), (String, Option<i64>)>,
) -> String {
    let mut out = String::new();

    for msg in messages {
        let role = match msg.role {
            crate::agent::types::MessageRole::User => "user",
            crate::agent::types::MessageRole::Assistant => "assistant",
        };

        out.push_str(&format!("[{role}] "));

        for block in &msg.blocks {
            match block {
                Block::UserText(b) => {
                    out.push_str(&b.content);
                    out.push('\n');
                }
                Block::Text(b) => {
                    out.push_str(&b.content);
                    out.push('\n');
                }
                Block::Tool(tool) => {
                    if matches!(tool.status, ToolStatus::Running) {
                        continue;
                    }
                    let key = (msg.id, tool.id.clone());
                    let compacted = tool.compacted_at.is_some()
                        || tool_outputs.get(&key).and_then(|(_, ts)| *ts).is_some();
                    let content = if compacted {
                        "[tool result cleared]".to_string()
                    } else {
                        tool_outputs
                            .get(&key)
                            .map(|(c, _)| c.clone())
                            .unwrap_or_else(|| String::new())
                    };
                    out.push_str(&format!("tool {}({}): {}\n", tool.name, tool.id, content));
                }
                _ => {}
            }
        }
        out.push('\n');
    }

    out
}
