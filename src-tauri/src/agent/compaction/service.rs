use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::agent::error::AgentResult;
use crate::agent::common::llm_text::extract_text_from_llm_message;
use crate::agent::persistence::AgentPersistence;
use crate::agent::prompt::BuiltinPrompts;
use crate::agent::types::{Block, Message, MessageRole, MessageStatus, TextBlock};
use crate::llm::anthropic_types::{
    CreateMessageRequest, MessageContent, MessageParam, SystemPrompt,
};
use crate::llm::service::LLMService;
use crate::storage::DatabaseManager;

use super::config::CompactionConfig;

#[derive(Debug, Clone, Copy)]
pub enum CompactionTrigger {
    Auto,
    Manual,
}

pub struct PreparedCompaction {
    pub summary_job: Option<SummaryJob>,
}

pub struct SummaryJob {
    pub summary_message: Message,
    pub source_text: String,
}

pub struct SummaryCompletion {
    pub message_id: i64,
    pub status: MessageStatus,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: i64,
}

pub struct CompactionService {
    database: Arc<DatabaseManager>,
    persistence: Arc<AgentPersistence>,
    config: CompactionConfig,
}

impl CompactionService {
    pub fn new(
        database: Arc<DatabaseManager>,
        persistence: Arc<AgentPersistence>,
        config: CompactionConfig,
    ) -> Self {
        Self {
            database,
            persistence,
            config,
        }
    }

    pub async fn prepare_compaction(
        &self,
        session_id: i64,
        _context_window: u32,
        _trigger: CompactionTrigger,
    ) -> AgentResult<PreparedCompaction> {
        if !self.config.enabled {
            return Ok(PreparedCompaction { summary_job: None });
        }

        let messages = self
            .persistence
            .messages()
            .list_by_session(session_id)
            .await?;
        if messages.len() < self.config.min_messages as usize {
            return Ok(PreparedCompaction { summary_job: None });
        }

        let last_summary_idx = messages
            .iter()
            .rposition(|m| m.is_summary && matches!(m.status, MessageStatus::Completed));
        let start_idx = last_summary_idx.map(|idx| idx + 1).unwrap_or(0);
        let unsummarized = messages.len().saturating_sub(start_idx);
        if unsummarized <= self.config.max_unsummarized_messages as usize {
            return Ok(PreparedCompaction { summary_job: None });
        }

        let keep = self.config.keep_recent_messages as usize;
        let tail_start_idx = messages.len().saturating_sub(keep).max(start_idx);
        if tail_start_idx <= start_idx {
            return Ok(PreparedCompaction { summary_job: None });
        }

        let tail_ts = messages[tail_start_idx].created_at.timestamp();
        let summary_created_at = tail_ts.saturating_sub(1);

        let source_text = build_summary_source(&messages[start_idx..tail_start_idx]);
        if source_text.trim().is_empty() {
            return Ok(PreparedCompaction { summary_job: None });
        }

        let agent_type = messages
            .last()
            .map(|m| m.agent_type.clone())
            .unwrap_or_else(|| "coder".to_string());

        let summary_message = self
            .persistence
            .messages()
            .create_summary_message(session_id, &agent_type, summary_created_at)
            .await?;

        Ok(PreparedCompaction {
            summary_job: Some(SummaryJob {
                summary_message,
                source_text,
            }),
        })
    }

    pub async fn complete_summary_job(
        &self,
        job: SummaryJob,
        model_id: &str,
    ) -> AgentResult<SummaryCompletion> {
        let start = Utc::now();

        let system = SystemPrompt::Text(BuiltinPrompts::system_compaction().to_string());

        let prompt =
            BuiltinPrompts::system_compaction_user().replace("{{transcript}}", &job.source_text);

        let request = CreateMessageRequest {
            model: model_id.to_string(),
            max_tokens: 1024,
            system: Some(system),
            messages: vec![MessageParam {
                role: crate::llm::anthropic_types::MessageRole::User,
                content: MessageContent::Text(prompt),
            }],
            tools: None,
            stream: false,
            temperature: Some(0.2),
            top_p: None,
            top_k: None,
            metadata: None,
            stop_sequences: None,
        };

        let llm = LLMService::new(Arc::clone(&self.database));
        let resp = llm.call(request).await.map_err(|e| {
            crate::agent::error::AgentError::Internal(format!("Compaction LLM call failed: {e}"))
        })?;

        let summary = crate::agent::common::truncate_chars_no_ellipsis(
            &extract_text_from_llm_message(&resp),
            self.config.max_summary_chars as usize,
        );

        let finished_at = Utc::now();
        let duration_ms = finished_at
            .signed_duration_since(start)
            .num_milliseconds()
            .max(0);

        let mut summary_message = job.summary_message.clone();
        summary_message.blocks = vec![Block::Text(TextBlock {
            id: uuid::Uuid::new_v4().to_string(),
            content: summary,
            is_streaming: false,
        })];
        summary_message.status = MessageStatus::Completed;
        summary_message.finished_at = Some(finished_at);
        summary_message.duration_ms = Some(duration_ms);
        summary_message.role = MessageRole::Assistant;

        self.persistence.messages().update(&summary_message).await?;

        Ok(SummaryCompletion {
            message_id: summary_message.id,
            status: MessageStatus::Completed,
            finished_at,
            duration_ms,
        })
    }
}

fn build_summary_source(messages: &[Message]) -> String {
    let mut out = Vec::new();
    for msg in messages {
        let role = match msg.role {
            MessageRole::User => "USER",
            MessageRole::Assistant => "ASSISTANT",
        };
        let text = extract_message_text(msg);
        if text.trim().is_empty() {
            continue;
        }
        out.push(format!("{role}:\n{text}"));
    }
    out.join("\n\n").trim().to_string()
}

fn extract_message_text(message: &Message) -> String {
    let mut parts = Vec::new();
    match message.role {
        MessageRole::User => {
            for block in &message.blocks {
                if let Block::UserText(b) = block {
                    if !b.content.trim().is_empty() {
                        parts.push(b.content.trim().to_string());
                    }
                }
            }
        }
        MessageRole::Assistant => {
            for block in &message.blocks {
                match block {
                    Block::Text(b) => {
                        if !b.content.trim().is_empty() {
                            parts.push(b.content.trim().to_string());
                        }
                    }
                    Block::Subtask(b) => {
                        if let Some(summary) = &b.summary {
                            if !summary.trim().is_empty() {
                                parts.push(summary.trim().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    parts.join("\n")
}
