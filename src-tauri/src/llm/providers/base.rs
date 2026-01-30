use async_trait::async_trait;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::anthropic_types::{CreateMessageRequest, Message, StreamEvent};
use crate::llm::error::{LlmProviderError, LlmProviderResult};
use crate::llm::types::{EmbeddingRequest, EmbeddingResponse};

/// LLM Provider 统一接口
///
/// "Never break userspace": 对调用方来说，只看到Anthropic类型。
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// 非流式调用
    ///
    /// 接受 Anthropic CreateMessageRequest，返回 Anthropic Message。
    /// 其他provider需要内部转换，但接口必须是纯Anthropic类型。
    async fn call(&self, request: CreateMessageRequest) -> LlmProviderResult<Message>;

    /// 流式调用
    ///
    /// 接受 Anthropic CreateMessageRequest，返回 Anthropic StreamEvent流。
    ///
    /// 流式事件包括：
    /// - MessageStart: 消息开始，包含初始usage
    /// - ContentBlockStart/Delta/Stop: 内容块（文本/工具调用）
    /// - MessageDelta: 消息级别更新（stop_reason等）
    /// - MessageStop: 完成
    async fn call_stream(
        &self,
        request: CreateMessageRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<StreamEvent>> + Send>>>;

    /// Embedding调用
    ///
    /// 生成文本的向量表示，用于语义搜索和相似度计算。
    /// 如果provider不支持embedding，应返回NotImplemented错误。
    ///
    /// 注意：Embedding不使用Anthropic类型，因为Anthropic不提供embedding API。
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
