//! Anthropic Provider - 直接使用 Anthropic 原生类型
//!
//!
//! ## 使用示例
//!
//! ```rust
//! use orbitx::llm::anthropic_types::*;
//! use orbitx::llm::providers::AnthropicProvider;
//! use orbitx::llm::types::LLMProviderConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = LLMProviderConfig {
//!     provider_type: "anthropic".to_string(),
//!     api_key: "your-api-key".to_string(),
//!     api_url: None,
//!     model: "claude-3-5-sonnet-20241022".to_string(),
//!     options: None,
//! };
//!
//! let provider = AnthropicProvider::new(config);
//! let request = CreateMessageRequest { /* ... */ };
//! let message = provider.call(request).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::Stream;

use crate::llm::anthropic_types::*;
use crate::llm::error::{AnthropicError, LlmProviderError, LlmProviderResult};
use crate::llm::providers::base::LLMProvider;
use crate::llm::types::LLMProviderConfig;

type AnthropicResult<T> = Result<T, AnthropicError>;

/// 全局共享的HTTP客户端，优化连接复用
static SHARED_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create shared HTTP client")
});

/// Anthropic Provider
///
/// 直接使用 Anthropic API 的原生类型，无中间转换层
/// 使用全局共享HTTP客户端优化性能
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// 从配置创建新的 Anthropic provider
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            api_key: config.api_key,
            base_url: config
                .api_url
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
        }
    }

    /// 获取共享HTTP客户端
    fn client(&self) -> &'static Client {
        &SHARED_HTTP_CLIENT
    }

    /// 获取 API endpoint
    fn get_endpoint(&self) -> String {
        format!("{}/messages", self.base_url)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-api-key", self.api_key.parse().unwrap());
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());
        headers
    }

    /// 内部非流式调用实现
    async fn call_internal(&self, request: CreateMessageRequest) -> AnthropicResult<Message> {
        let response = self
            .client()
            .post(self.get_endpoint())
            .headers(self.build_headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| AnthropicError::Http { source: e })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AnthropicError::Http { source: e })?;

        if !status.is_success() {
            return Err(self.parse_error_response(status, &body));
        }

        serde_json::from_str(&body).map_err(|e| AnthropicError::Json { source: e })
    }

    /// 内部流式调用实现
    async fn call_stream_internal(
        &self,
        mut request: CreateMessageRequest,
    ) -> AnthropicResult<Pin<Box<dyn Stream<Item = AnthropicResult<StreamEvent>> + Send>>> {
        // 强制开启流式
        request.stream = true;

        let response = self
            .client()
            .post(self.get_endpoint())
            .headers(self.build_headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| AnthropicError::Http { source: e })?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| AnthropicError::Http { source: e })?;
            return Err(self.parse_error_response(status, &body));
        }

        // 解析 SSE 流
        let event_stream = response
            .bytes_stream()
            .eventsource()
            .map(|result| match result {
                Ok(event) => {
                    if event.event == "message_stop" || event.event == "ping" {
                        // 这些事件可能没有 data 字段
                        match event.event.as_str() {
                            "message_stop" => Ok(StreamEvent::MessageStop),
                            "ping" => Ok(StreamEvent::Ping),
                            _ => Err(AnthropicError::Stream {
                                message: "Unknown event type".to_string(),
                            }),
                        }
                    } else {
                        // 解析 JSON data
                        serde_json::from_str::<StreamEvent>(&event.data)
                            .map_err(|e| AnthropicError::Json { source: e })
                    }
                }
                Err(e) => Err(AnthropicError::Stream {
                    message: e.to_string(),
                }),
            });

        Ok(Box::pin(event_stream))
    }

    /// 解析错误响应
    fn parse_error_response(&self, status: StatusCode, body: &str) -> AnthropicError {
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(error_obj) = error_json["error"].as_object() {
                let error_message = error_obj["message"].as_str().unwrap_or("Unknown error");

                return AnthropicError::Api {
                    status,
                    message: error_message.to_string(),
                };
            }
        }

        AnthropicError::Api {
            status,
            message: body.to_string(),
        }
    }
}

// ============================================================
// 辅助函数：应用 Prompt Caching
// ============================================================

/// 为支持缓存的模型添加 cache_control 标记
///
/// Anthropic 推荐的缓存策略：
/// 1. System prompt 添加缓存
/// 2. 最近两条 user 消息添加缓存
///

pub fn apply_prompt_caching(mut request: CreateMessageRequest) -> CreateMessageRequest {
    // 1. 给 system prompt 添加缓存控制
    if let Some(SystemPrompt::Text(text)) = request.system.take() {
        request.system = Some(SystemPrompt::Blocks(vec![SystemBlock {
            block_type: "text".to_string(),
            text,
            cache_control: Some(CacheControl::ephemeral()),
        }]));
    } else if let Some(SystemPrompt::Blocks(mut blocks)) = request.system.take() {
        if let Some(last_block) = blocks.last_mut() {
            last_block.cache_control = Some(CacheControl::ephemeral());
        }
        request.system = Some(SystemPrompt::Blocks(blocks));
    }

    // 2. 找到最后两条 user 消息
    let user_indices: Vec<usize> = request
        .messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| msg.role == MessageRole::User)
        .map(|(idx, _)| idx)
        .collect();

    let last_user_idx = user_indices.last().copied();
    let second_last_user_idx = if user_indices.len() >= 2 {
        Some(user_indices[user_indices.len() - 2])
    } else {
        None
    };

    // 3. 给这两条消息的最后一个 block 添加缓存控制
    for (idx, msg) in request.messages.iter_mut().enumerate() {
        if Some(idx) == last_user_idx || Some(idx) == second_last_user_idx {
            if let MessageContent::Blocks(blocks) = &mut msg.content {
                if let Some(last_block) = blocks.last_mut() {
                    match last_block {
                        ContentBlock::Text { cache_control, .. } => {
                            *cache_control = Some(CacheControl::ephemeral());
                        }
                        ContentBlock::Image { cache_control, .. } => {
                            *cache_control = Some(CacheControl::ephemeral());
                        }
                        _ => {}
                    }
                }
            } else if let MessageContent::Text(text) = &msg.content {
                // 将纯文本转为带缓存的块
                msg.content = MessageContent::Blocks(vec![ContentBlock::Text {
                    text: text.clone(),
                    cache_control: Some(CacheControl::ephemeral()),
                }]);
            }
        }
    }

    request
}

// ============================================================
// LLMProvider Trait 实现
// ============================================================

#[async_trait]
impl LLMProvider for AnthropicProvider {
    /// 非流式调用 - 直接返回Anthropic原生类型
    async fn call(&self, request: CreateMessageRequest) -> LlmProviderResult<Message> {
        self.call_internal(request)
            .await
            .map_err(LlmProviderError::from)
    }

    /// 流式调用 - 直接返回Anthropic StreamEvent流
    async fn call_stream(
        &self,
        request: CreateMessageRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<StreamEvent>> + Send>>> {
        // 调用内部实现
        let stream = self
            .call_stream_internal(request)
            .await
            .map_err(LlmProviderError::from)?;

        // 转换错误类型：AnthropicError -> LlmProviderError
        let converted_stream = stream.map(|result| result.map_err(LlmProviderError::from));

        Ok(Box::pin(converted_stream))
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_prompt_caching_to_system() {
        let request = CreateMessageRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![MessageParam::user("Hello")],
            max_tokens: 1024,
            system: Some(SystemPrompt::Text("You are helpful".to_string())),
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
        };

        let cached_request = apply_prompt_caching(request);

        match cached_request.system {
            Some(SystemPrompt::Blocks(blocks)) => {
                assert_eq!(blocks.len(), 1);
                assert!(blocks[0].cache_control.is_some());
            }
            _ => panic!("Expected system blocks with cache control"),
        }
    }

    #[test]
    fn test_apply_prompt_caching_to_last_two_user_messages() {
        let request = CreateMessageRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![
                MessageParam::user("First message"),
                MessageParam::assistant("Response 1"),
                MessageParam::user("Second message"),
                MessageParam::assistant("Response 2"),
                MessageParam::user("Third message"),
            ],
            max_tokens: 1024,
            system: None,
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
        };

        let cached_request = apply_prompt_caching(request);

        // 检查第2条和第3条 user 消息（索引2和4）
        for (idx, msg) in cached_request.messages.iter().enumerate() {
            if msg.role == MessageRole::User {
                if let MessageContent::Blocks(blocks) = &msg.content {
                    if let Some(ContentBlock::Text { cache_control, .. }) = blocks.last() {
                        if idx == 2 || idx == 4 {
                            assert!(
                                cache_control.is_some(),
                                "Last two user messages should have cache control"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_build_headers() {
        use crate::llm::types::LLMProviderConfig;

        let config = LLMProviderConfig {
            provider_type: "anthropic".to_string(),
            api_key: "test-key".to_string(),
            api_url: None,
            options: None,
        };

        let provider = AnthropicProvider::new(config);
        let headers = provider.build_headers();

        assert_eq!(headers.get("x-api-key").unwrap(), "test-key");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }
}
