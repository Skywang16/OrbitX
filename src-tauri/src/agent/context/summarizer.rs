use std::sync::Arc;

use tracing::warn;

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::persistence::{AgentPersistence, ConversationSummary};
use crate::agent::prompt::components::conversation_summary::{
    build_conversation_summary_user_prompt, CONVERSATION_SUMMARY_SYSTEM_PROMPT,
};
use crate::llm::anthropic_types::{ContentBlock, CreateMessageRequest, MessageContent, MessageParam, SystemPrompt};
use crate::llm::service::LLMService;
use crate::storage::DatabaseManager;

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
    repositories: Arc<DatabaseManager>,
}

impl ConversationSummarizer {
    pub fn new(
        conversation_id: i64,
        persistence: Arc<AgentPersistence>,
        repositories: Arc<DatabaseManager>,
    ) -> Self {
        Self {
            conversation_id,
            persistence,
            repositories,
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
        messages: &[MessageParam],
        _system: &Option<SystemPrompt>,
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
        messages: &[MessageParam],
        _system: &Option<SystemPrompt>,
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
        messages: &[MessageParam],
        current_tokens: u32,
    ) -> AgentResult<SummaryResult> {
        let (summary_scope, recent_tail) = split_messages(messages, RECENT_MESSAGES_TO_KEEP);
        if summary_scope.is_empty() {
            return Err(AgentError::Internal(
                "No conversation history available for summarization".to_string(),
            ));
        }

        // 构建 Anthropic 请求：system + user(prompt)
        let request = self.build_summary_request(model_id, &summary_scope);

        // LLMService 会在内部将 model 字段转换为 provider-specific 名称
        let llm_service = LLMService::new(self.repositories());
        let response = llm_service.call(request.clone()).await.map_err(|e| {
            AgentError::Internal(format!("Failed to call LLM for summary generation: {}", e))
        })?;

        let summary_text = render_content_blocks(&response.content);
        if summary_text.trim().is_empty() {
            return Err(AgentError::Internal("LLM summary is empty".to_string()));
        }

        let summary_tokens = response.usage.output_tokens;
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
            summary: summary_text.trim().to_string(),
            token_count: summary_tokens,
            cost: extract_cost_from_usage(response.usage.total_tokens()),
            prev_context_tokens: current_tokens,
            messages_before_summary: summary_scope.len(),
            tokens_saved,
        })
    }

    fn build_summary_request(&self, model_id: &str, summary_scope: &[MessageParam]) -> CreateMessageRequest {
        let mut history_builder = String::new();
        for message in summary_scope {
            history_builder.push_str(&format!(
                "[{role}] {content}\n\n",
                role = match message.role { crate::llm::anthropic_types::MessageRole::User => "user", crate::llm::anthropic_types::MessageRole::Assistant => "assistant" },
                content = render_message_param_content(&message.content)
            ));
        }

        CreateMessageRequest {
            model: model_id.to_string(),
            messages: vec![MessageParam { role: crate::llm::anthropic_types::MessageRole::User, content: MessageContent::Text(build_conversation_summary_user_prompt(&history_builder)) }],
            max_tokens: SUMMARY_MAX_TOKENS,
            system: Some(SystemPrompt::Text(CONVERSATION_SUMMARY_SYSTEM_PROMPT.to_string())),
            tools: None,
            temperature: Some(0.3),
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
        }
    }

    fn fallback_to_sliding_window(
        &self,
        messages: &[MessageParam],
        current_tokens: u32,
    ) -> SummaryResult {
        let (summary_scope, _) = split_messages(messages, RECENT_MESSAGES_TO_KEEP);
        let mut text = String::new();
        for message in summary_scope.iter().take(RECENT_MESSAGES_TO_KEEP * 4) {
            text.push_str(&format!(
                "[{role:?}] {content}\n",
                role = message.role,
                content = render_message_param_content(&message.content)
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
            token_count: estimate_text_tokens(&text),
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
        // 同步获取模型配置中的 context window
        let model_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::storage::repositories::AIModels::new(&self.repositories)
                    .find_by_id(model_id)
                    .await
            })
        });

        if let Ok(Some(model)) = model_result {
            if let Some(options) = model.options {
                if let Some(max_tokens) = options.get("maxContextTokens") {
                    if let Some(value) = max_tokens.as_u64() {
                        return value as u32;
                    }
                }
            }
        }

        // 默认值：大多数现代 LLM 支持 128K tokens
        128_000
    }
}

fn split_messages<'a>(
    messages: &'a [MessageParam],
    tail_keep: usize,
) -> (Vec<MessageParam>, Vec<MessageParam>) {
    if messages.len() <= tail_keep {
        return (Vec::new(), messages.to_vec());
    }
    let split_at = messages.len() - tail_keep;
    let summary_scope = messages[..split_at].to_vec();
    let recent_tail = messages[split_at..].to_vec();
    (summary_scope, recent_tail)
}

fn estimate_messages_tokens(messages: &[MessageParam]) -> u32 {
    use crate::agent::utils::tokenizer::count_message_param_tokens;
    messages
        .iter()
        .map(|m| count_message_param_tokens(m) as u32)
        .fold(0u32, |acc, n| acc.saturating_add(n))
}

fn estimate_text_tokens(text: &str) -> u32 {
    ((text.len() as f32) / 4.0).ceil() as u32
}

fn render_message_param_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.trim().to_string(),
        MessageContent::Blocks(blocks) => render_content_blocks(blocks),
    }
}

fn render_content_blocks(blocks: &Vec<ContentBlock>) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            ContentBlock::Text { text, .. } => {
                if !out.is_empty() { out.push_str("\n"); }
                out.push_str(text);
            }
            ContentBlock::Thinking { thinking, .. } => {
                // 不放入用户可见摘要
                let _ = thinking; // 忽略
            }
            other => {
                // 对于非文本块，简化为 JSON 摘要
                let s = serde_json::to_string(other).unwrap_or_default();
                if !out.is_empty() { out.push_str("\n"); }
                out.push_str(&s);
            }
        }
    }
    out
}

fn extract_cost_from_usage(total_tokens: u32) -> f64 {
    // 占位：按 token 数乘以统一单价估算
    (total_tokens as f64) * 0.000_002
}

impl ConversationSummarizer {
    fn repositories(&self) -> Arc<DatabaseManager> {
        Arc::clone(&self.repositories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_messages_handles_short_history() {
        let messages = vec![
            MessageParam { role: crate::llm::anthropic_types::MessageRole::User, content: MessageContent::Text("hello".into()) },
            MessageParam { role: crate::llm::anthropic_types::MessageRole::Assistant, content: MessageContent::Text("world".into()) },
        ];

        let (summary, tail) = split_messages(&messages, 3);
        assert!(summary.is_empty());
        assert_eq!(tail.len(), 2);
    }

    #[test]
    fn split_messages_keeps_tail() {
        let messages = (0..5)
            .map(|i| MessageParam { role: if i % 2 == 0 { crate::llm::anthropic_types::MessageRole::User } else { crate::llm::anthropic_types::MessageRole::Assistant }, content: MessageContent::Text(format!("m{}", i)) })
            .collect::<Vec<_>>();

        let (summary, tail) = split_messages(&messages, 3);
        assert_eq!(summary.len(), 2);
        assert_eq!(tail.len(), 3);
    }
}
