use async_trait::async_trait;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult},
    providers::base::LLMProvider,
    types::LLMProviderConfig,
};

/// Gemini Provider (messages unsupported in zero-abstraction mode)
pub struct GeminiProvider;

impl GeminiProvider {
    pub fn new(_config: LLMProviderConfig) -> Self {
        Self
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn call(
        &self,
        _request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<crate::llm::anthropic_types::Message> {
        Err(LlmProviderError::UnsupportedOperation {
            provider: "gemini",
            operation: "messages",
        })
    }

    async fn call_stream(
        &self,
        _request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<
        Pin<
            Box<
                dyn Stream<Item = LlmProviderResult<crate::llm::anthropic_types::StreamEvent>>
                    + Send,
            >,
        >,
    > {
        Err(LlmProviderError::UnsupportedOperation {
            provider: "gemini",
            operation: "messages_stream",
        })
    }
}
