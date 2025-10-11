use std::sync::Arc;

use tracing::warn;

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::persistence::{AgentPersistence, ConversationSummary};
use crate::agent::prompt::components::conversation_summary::{
    build_conversation_summary_user_prompt, CONVERSATION_SUMMARY_SYSTEM_PROMPT,
};
use crate::llm::registry::LLMRegistry;
use crate::llm::service::LLMService;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMRequest, LLMResponse};
use crate::storage::repositories::RepositoryManager;

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
    ) -> AgentResult<Option<SummaryResult>> {
        if messages.is_empty() {
            return Ok(None);
        }

        let context_window = self.lookup_context_window(model_id);
        let current_tokens = estimate_messages_tokens(messages);

        if !self.should_compress(current_tokens, context_window) {
            return Ok(None);
        }

        match self
            .summarize_conversation(model_id, messages, current_tokens)
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
    ) -> AgentResult<SummaryResult> {
        if messages.is_empty() {
            return Err(AgentError::Internal(
                "No conversation history available for summarization".to_string(),
            ));
        }

        let current_tokens = estimate_messages_tokens(messages);

        match self
            .summarize_conversation(model_id, messages, current_tokens)
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
        current_tokens: u32,
    ) -> AgentResult<SummaryResult> {
        let (summary_scope, recent_tail) = split_messages(messages, RECENT_MESSAGES_TO_KEEP);
        if summary_scope.is_empty() {
            return Err(AgentError::Internal(
                "No conversation history available for summarization".to_string(),
            ));
        }

        let prompt_messages = self.build_summary_prompt(&summary_scope);
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
        let response = llm_service.call(request.clone()).await.map_err(|e| {
            AgentError::Internal(format!("Failed to call LLM for summary generation: {}", e))
        })?;

        if response.content.trim().is_empty() {
            return Err(AgentError::Internal("LLM summary is empty".to_string()));
        }

        let summary_tokens = response
            .usage
            .as_ref()
            .map(|usage| usage.completion_tokens)
            .unwrap_or_else(|| estimate_tokens(&response.content));
        let recent_tokens = estimate_messages_tokens(&recent_tail);
        let new_context_tokens = summary_tokens + recent_tokens;

        // 关键验证：总结后的 token 数不能大于原来的
        if new_context_tokens >= current_tokens {
            return Err(AgentError::Internal(format!(
                "Summary failed: new context ({} tokens) is not smaller than original ({} tokens). Summary might be too verbose.",
                new_context_tokens, current_tokens
            )));
        }

        let tokens_saved = current_tokens.saturating_sub(new_context_tokens);

        Ok(SummaryResult {
            summary: response.content.trim().to_string(),
            token_count: summary_tokens,
            cost: extract_cost(&response),
            prev_context_tokens: current_tokens,
            messages_before_summary: summary_scope.len(),
            tokens_saved,
        })
    }

    fn build_summary_prompt(&self, summary_scope: &[LLMMessage]) -> Vec<LLMMessage> {
        let mut history_builder = String::new();
        for message in summary_scope {
            history_builder.push_str(&format!(
                "[{role}] {content}\n\n",
                role = message.role,
                content = render_message_content(&message.content)
            ));
        }

        vec![
            LLMMessage {
                role: "system".to_string(),
                content: LLMMessageContent::Text(CONVERSATION_SUMMARY_SYSTEM_PROMPT.to_string()),
            },
            LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text(build_conversation_summary_user_prompt(
                    &history_builder,
                )),
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

    async fn persist_summary(&self, result: &SummaryResult) -> AgentResult<ConversationSummary> {
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
    fn repositories(&self) -> Arc<RepositoryManager> {
        Arc::clone(&self.repositories)
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
