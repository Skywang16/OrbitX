use std::sync::Arc;

use anyhow::anyhow;
use tracing::warn;

use crate::agent::persistence::{AgentPersistence, ConversationSummary};
use crate::llm::registry::LLMRegistry;
use crate::llm::service::LLMService;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMRequest, LLMResponse};
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::AppResult;

const COMPRESSION_THRESHOLD: f32 = 0.85;
const SUMMARY_MAX_TOKENS: u32 = 512;
const RECENT_MESSAGES_TO_KEEP: usize = 3;

#[derive(Debug, Clone)]
pub struct SummaryResult {
    pub summary: String,
    pub token_count: u32,
    pub cost: f64,
    pub prev_context_tokens: u32,
    pub messages_before_summary: usize,
    pub tokens_saved: u32,
}

pub struct ConversationSummarizer {
    conversation_id: i64,
    persistence: Arc<AgentPersistence>,
    repositories: Arc<RepositoryManager>,
    llm_registry: Arc<LLMRegistry>,
}

impl ConversationSummarizer {
    pub fn new(
        conversation_id: i64,
        persistence: Arc<AgentPersistence>,
        repositories: Arc<RepositoryManager>,
        llm_registry: Arc<LLMRegistry>,
    ) -> Self {
        Self {
            conversation_id,
            persistence,
            repositories,
            llm_registry,
        }
    }

    pub fn should_compress(&self, current_tokens: u32, context_window: u32) -> bool {
        if context_window == 0 {
            return false;
        }
        (current_tokens as f32) >= (context_window as f32 * COMPRESSION_THRESHOLD)
    }

    pub async fn summarize_if_needed(
        &self,
        model_id: &str,
        messages: &[LLMMessage],
    ) -> AppResult<Option<SummaryResult>> {
        if messages.is_empty() {
            return Ok(None);
        }

        let context_window = self.lookup_context_window(model_id);
        let current_tokens = estimate_messages_tokens(messages);

        if !self.should_compress(current_tokens, context_window) {
            return Ok(None);
        }

        match self
            .summarize_conversation(model_id, messages, context_window, current_tokens)
            .await
        {
            Ok(result) => {
                self.persist_summary(&result).await?;
                Ok(Some(result))
            }
            Err(err) => {
                warn!(
                    "Conversation summarization failed for conversation {}: {}. Falling back to sliding window.",
                    self.conversation_id, err
                );
                let fallback = self.fallback_to_sliding_window(messages, current_tokens);
                self.persist_summary(&fallback).await?;
                Ok(Some(fallback))
            }
        }
    }

    pub async fn summarize_now(
        &self,
        model_id: &str,
        messages: &[LLMMessage],
    ) -> AppResult<SummaryResult> {
        if messages.is_empty() {
            return Err(anyhow!("没有可用于摘要的历史消息"));
        }

        let context_window = self.lookup_context_window(model_id);
        let current_tokens = estimate_messages_tokens(messages);

        match self
            .summarize_conversation(model_id, messages, context_window, current_tokens)
            .await
        {
            Ok(result) => {
                self.persist_summary(&result).await?;
                Ok(result)
            }
            Err(err) => {
                warn!(
                    "Manual conversation summarization failed for conversation {}: {}. Using fallback window.",
                    self.conversation_id, err
                );
                let fallback = self.fallback_to_sliding_window(messages, current_tokens);
                self.persist_summary(&fallback).await?;
                Ok(fallback)
            }
        }
    }

    async fn summarize_conversation(
        &self,
        model_id: &str,
        messages: &[LLMMessage],
        _context_window: u32,
        current_tokens: u32,
    ) -> AppResult<SummaryResult> {
        let (summary_scope, recent_tail) = split_messages(messages, RECENT_MESSAGES_TO_KEEP);
        if summary_scope.is_empty() {
            return Err(anyhow!("没有可用于摘要的历史消息"));
        }

        let prompt_messages = self.build_summary_prompt(&summary_scope, &recent_tail);
        let request = LLMRequest {
            model: model_id.to_string(),
            messages: prompt_messages,
            temperature: Some(0.3),
            max_tokens: Some(SUMMARY_MAX_TOKENS),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        // LLMService 会在内部将 model 字段转换为 provider-specific 名称
        let llm_service = LLMService::new(self.repositories());
        let response = llm_service
            .call(request.clone())
            .await
            .map_err(|e| anyhow!("调用LLM生成摘要失败: {}", e))?;

        if response.content.trim().is_empty() {
            return Err(anyhow!("LLM摘要为空"));
        }

        let summary_tokens = response
            .usage
            .as_ref()
            .map(|usage| usage.completion_tokens)
            .unwrap_or_else(|| estimate_tokens(&response.content));
        let recent_tokens = estimate_messages_tokens(&recent_tail);
        let tokens_saved = current_tokens.saturating_sub(summary_tokens + recent_tokens);

        Ok(SummaryResult {
            summary: response.content.trim().to_string(),
            token_count: summary_tokens,
            cost: extract_cost(&response),
            prev_context_tokens: current_tokens,
            messages_before_summary: summary_scope.len(),
            tokens_saved,
        })
    }

    fn build_summary_prompt(
        &self,
        summary_scope: &[LLMMessage],
        recent_tail: &[LLMMessage],
    ) -> Vec<LLMMessage> {
        let mut builder = String::new();
        builder.push_str(
            "Summarize the following conversation history for an autonomous coding agent.\n",
        );
        builder.push_str(
            "Focus on goals, constraints, tools used, files touched, and unresolved items.\n",
        );
        builder.push_str("Return a concise markdown bullet list.\n\n");
        builder.push_str("<history>\n");
        for message in summary_scope {
            builder.push_str(&format!(
                "[{role}] {content}\n",
                role = message.role,
                content = render_message_content(&message.content)
            ));
        }
        builder.push_str("</history>\n");

        if !recent_tail.is_empty() {
            builder.push_str("\nRecent messages that must remain verbatim:\n");
            for (idx, m) in recent_tail.iter().enumerate() {
                builder.push_str(&format!(
                    "- #{index} [{role}] {content}\n",
                    index = idx + 1,
                    role = m.role,
                    content = render_message_content(&m.content)
                ));
            }
        }

        vec![
            LLMMessage {
                role: "system".to_string(),
                content: LLMMessageContent::Text(
                    "You compress conversation history into a durable memory for an autonomous agent.
Include key facts, open TODOs, and file references. Do not invent details.".to_string(),
                ),
            },
            LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text(builder),
            },
        ]
    }

    fn fallback_to_sliding_window(
        &self,
        messages: &[LLMMessage],
        current_tokens: u32,
    ) -> SummaryResult {
        let (summary_scope, _) = split_messages(messages, RECENT_MESSAGES_TO_KEEP);
        let mut text = String::new();
        for message in summary_scope.iter().take(RECENT_MESSAGES_TO_KEEP * 4) {
            text.push_str(&format!(
                "[{role}] {content}\n",
                role = message.role,
                content = render_message_content(&message.content)
            ));
        }
        if text.len() > 2000 {
            text.truncate(2000);
            text.push_str("... (truncated)\n");
        }

        SummaryResult {
            summary: format!(
                "Failed to compress via LLM. Retained leading context:\n{}",
                text
            ),
            token_count: estimate_tokens(&text),
            cost: 0.0,
            prev_context_tokens: current_tokens,
            messages_before_summary: summary_scope.len(),
            tokens_saved: 0,
        }
    }

    async fn persist_summary(&self, result: &SummaryResult) -> AppResult<ConversationSummary> {
        let repo = self.persistence.conversation_summaries();
        repo.upsert(
            self.conversation_id,
            &result.summary,
            result.token_count as i64,
            result.messages_before_summary as i64,
            result.tokens_saved as i64,
            result.cost,
        )
        .await
    }

    fn lookup_context_window(&self, model_id: &str) -> u32 {
        self.llm_registry
            .get_model_max_context(model_id)
            .unwrap_or(128_000)
    }
}

fn split_messages<'a>(
    messages: &'a [LLMMessage],
    tail_keep: usize,
) -> (Vec<LLMMessage>, Vec<LLMMessage>) {
    if messages.len() <= tail_keep {
        return (Vec::new(), messages.to_vec());
    }
    let split_at = messages.len() - tail_keep;
    let summary_scope = messages[..split_at].to_vec();
    let recent_tail = messages[split_at..].to_vec();
    (summary_scope, recent_tail)
}

fn estimate_messages_tokens(messages: &[LLMMessage]) -> u32 {
    messages
        .iter()
        .map(|m| estimate_tokens(&render_message_content(&m.content)))
        .sum()
}

fn estimate_tokens(text: &str) -> u32 {
    ((text.len() as f32) / 4.0).ceil() as u32
}

fn render_message_content(content: &LLMMessageContent) -> String {
    match content {
        LLMMessageContent::Text(text) => text.trim().to_string(),
        LLMMessageContent::Parts(parts) => serde_json::to_string(parts).unwrap_or_default(),
    }
}

fn extract_cost(response: &LLMResponse) -> f64 {
    // 多数供应商需要额外的元数据才能计算成本；此处占位，后续可根据模型信息完善
    response
        .usage
        .as_ref()
        .map(|usage| (usage.total_tokens as f64) * 0.000_002)
        .unwrap_or(0.0)
}

impl ConversationSummarizer {
    pub fn conversation_id(&self) -> i64 {
        self.conversation_id
    }

    pub fn persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.persistence)
    }

    pub fn repositories(&self) -> Arc<RepositoryManager> {
        Arc::clone(&self.repositories)
    }

    pub fn llm_registry(&self) -> Arc<LLMRegistry> {
        Arc::clone(&self.llm_registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_messages_handles_short_history() {
        let messages = vec![
            LLMMessage {
                role: "system".into(),
                content: LLMMessageContent::Text("hello".into()),
            },
            LLMMessage {
                role: "user".into(),
                content: LLMMessageContent::Text("world".into()),
            },
        ];

        let (summary, tail) = split_messages(&messages, 3);
        assert!(summary.is_empty());
        assert_eq!(tail.len(), 2);
    }

    #[test]
    fn split_messages_keeps_tail() {
        let messages = (0..5)
            .map(|i| LLMMessage {
                role: format!("r{}", i),
                content: LLMMessageContent::Text(format!("m{}", i)),
            })
            .collect::<Vec<_>>();

        let (summary, tail) = split_messages(&messages, 3);
        assert_eq!(summary.len(), 2);
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].role, "r2");
    }

    #[test]
    fn estimate_tokens_is_not_zero_for_content() {
        assert!(estimate_tokens("hello world") > 0);
    }
}
