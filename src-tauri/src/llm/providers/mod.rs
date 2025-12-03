pub mod anthropic;
pub mod base;
pub mod gemini;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use base::*;
pub use gemini::GeminiProvider;
pub use openai::OpenAIProvider;

use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    anthropic_types::{CreateMessageRequest, Message, StreamEvent},
    error::LlmProviderResult,
    types::{EmbeddingRequest, EmbeddingResponse},
};

/// Provider 枚举 - 零成本抽象，静态分发
///
/// 替代 Box<dyn LLMProvider>，消除 vtable 开销
/// 所有方法可内联，编译器完全优化
pub enum Provider {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
    Gemini(GeminiProvider),
}

impl Provider {
    /// 非流式调用 - 静态分发，可内联
    #[inline]
    pub async fn call(&self, request: CreateMessageRequest) -> LlmProviderResult<Message> {
        match self {
            Provider::OpenAI(p) => p.call(request).await,
            Provider::Anthropic(p) => p.call(request).await,
            Provider::Gemini(p) => p.call(request).await,
        }
    }

    /// 流式调用 - 静态分发
    #[inline]
    pub async fn call_stream(
        &self,
        request: CreateMessageRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<StreamEvent>> + Send>>> {
        match self {
            Provider::OpenAI(p) => p.call_stream(request).await,
            Provider::Anthropic(p) => p.call_stream(request).await,
            Provider::Gemini(p) => p.call_stream(request).await,
        }
    }

    /// Embedding 调用 - 静态分发
    #[inline]
    pub async fn create_embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> LlmProviderResult<EmbeddingResponse> {
        match self {
            Provider::OpenAI(p) => p.create_embeddings(request).await,
            Provider::Anthropic(p) => p.create_embeddings(request).await,
            Provider::Gemini(p) => p.create_embeddings(request).await,
        }
    }
}
