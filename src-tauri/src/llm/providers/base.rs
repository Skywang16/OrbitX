use async_trait::async_trait;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::types::{
    EmbeddingRequest, EmbeddingResponse, LLMRequest, LLMResponse, LLMStreamChunk,
};
use crate::llm::error::{LlmProviderError, LlmProviderResult};

/// LLM Provider 统一接口
///
/// 这个 Trait 定义了所有 LLM 提供商必须实现的统一接口，
/// 包括非流式和流式调用，以及embedding功能。
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// 非流式调用
    ///
    /// 发送一个完整的请求，并等待一个完整的响应。
    async fn call(&self, request: LLMRequest) -> LlmProviderResult<LLMResponse>;

    /// 流式调用
    ///
    /// 发送一个请求，并以流的形式接收响应数据块。
    /// 这对于需要实时反馈的场景非常有用。
    async fn call_stream(
        &self,
        request: LLMRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<LLMStreamChunk>> + Send>>>;

    /// Embedding调用
    ///
    /// 生成文本的向量表示，用于语义搜索和相似度计算。
    /// 如果provider不支持embedding，应返回NotImplemented错误。
    async fn create_embeddings(
        &self,
        _request: EmbeddingRequest,
    ) -> LlmProviderResult<EmbeddingResponse> {
        Err(LlmProviderError::UnsupportedOperation {
            provider: "unknown",
            operation: "embeddings",
        })
    }
}
