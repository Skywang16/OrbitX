use std::sync::Arc;

use crate::agent::error::{AgentError, AgentResult};
use crate::llm::anthropic_types::{
    CreateMessageRequest, MessageContent, MessageParam, MessageRole as AnthropicRole, SystemPrompt,
};
use crate::llm::service::LLMService;
use crate::storage::DatabaseManager;

use super::config::CompactionConfig;

const SUMMARY_SYSTEM_PROMPT: &str = r#"You are a summarization engine.
Summarize the conversation and important tool results for continuity.
Be concise, factual, and preserve decisions, constraints, open tasks, and file paths."#;

pub struct SessionCompactor {
    repositories: Arc<DatabaseManager>,
    config: CompactionConfig,
}

impl SessionCompactor {
    pub fn new(repositories: Arc<DatabaseManager>, config: CompactionConfig) -> Self {
        Self {
            repositories,
            config,
        }
    }

    pub async fn generate_summary(
        &self,
        model_id: &str,
        conversation_text: &str,
    ) -> AgentResult<String> {
        let model = self
            .config
            .summary_model
            .as_deref()
            .unwrap_or(model_id)
            .to_string();

        let req = CreateMessageRequest {
            model,
            max_tokens: self.config.summary_max_tokens,
            system: Some(SystemPrompt::Text(SUMMARY_SYSTEM_PROMPT.to_string())),
            messages: vec![MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Text(format!(
                    "Summarize the following conversation:\n\n{}",
                    conversation_text
                )),
            }],
            tools: None,
            temperature: Some(0.3),
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
        };

        let llm = LLMService::new(Arc::clone(&self.repositories));
        let resp = llm
            .call(req)
            .await
            .map_err(|e| AgentError::Internal(format!("Failed to call LLM for summary: {e}")))?;

        Ok(resp
            .content
            .iter()
            .filter_map(|c| match c {
                crate::llm::anthropic_types::ContentBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string())
    }
}
