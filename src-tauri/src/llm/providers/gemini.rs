use async_trait::async_trait;
use reqwest::Client;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult},
    providers::base::LLMProvider,
    types::LLMProviderConfig,
};

/// Gemini Provider (messages unsupported in zero-abstraction mode)
#[allow(dead_code)]
pub struct GeminiProvider {
    client: Client,
    config: LLMProviderConfig,
}

impl GeminiProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
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
