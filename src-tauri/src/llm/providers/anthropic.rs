//! Anthropic Provider - Directly uses Anthropic native types
//!
//!
//! ## Usage Examples
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
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client, StatusCode,
};
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::Stream;

use crate::llm::anthropic_types::*;
use crate::llm::error::{AnthropicError, LlmProviderError, LlmProviderResult};
use crate::llm::providers::base::LLMProvider;
use crate::llm::types::LLMProviderConfig;

type AnthropicResult<T> = Result<T, AnthropicError>;

/// Global shared HTTP client for optimized connection reuse
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
/// Directly uses Anthropic API native types, no intermediate conversion layer
/// Uses global shared HTTP client for performance optimization
#[derive(Clone)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from config
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            api_key: config.api_key,
            base_url: config
                .api_url
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
        }
    }

    /// Get shared HTTP client
    fn client(&self) -> &'static Client {
        &SHARED_HTTP_CLIENT
    }

    /// Get API endpoint
    fn get_endpoint(&self) -> String {
        format!("{}/messages", self.base_url)
    }

    /// Build request headers
    fn build_headers(&self) -> AnthropicResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        let api_key =
            HeaderValue::from_str(&self.api_key).map_err(|err| AnthropicError::Configuration {
                message: format!("Invalid Anthropic API key header value: {err}"),
            })?;
        headers.insert("x-api-key", api_key);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    /// Internal non-streaming call implementation
    async fn call_internal(&self, request: CreateMessageRequest) -> AnthropicResult<Message> {
        let response = self
            .client()
            .post(self.get_endpoint())
            .headers(self.build_headers()?)
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

        tracing::debug!(
            "Anthropic API response (truncated): {}",
            &body[..body.len().min(500)]
        );

        serde_json::from_str(&body).map_err(|e| {
            tracing::error!(
                "Failed to parse Anthropic response: {} | raw: {}",
                e,
                &body[..body.len().min(1000)]
            );
            AnthropicError::Json { source: e }
        })
    }

    /// Internal streaming call implementation
    async fn call_stream_internal(
        &self,
        mut request: CreateMessageRequest,
    ) -> AnthropicResult<Pin<Box<dyn Stream<Item = AnthropicResult<StreamEvent>> + Send>>> {
        // Force enable streaming
        request.stream = true;

        let response = self
            .client()
            .post(self.get_endpoint())
            .headers(self.build_headers()?)
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

        // Parse SSE stream
        let event_stream = response
            .bytes_stream()
            .eventsource()
            .map(|result| match result {
                Ok(event) => {
                    if event.event == "message_stop" || event.event == "ping" {
                        // These events may not have a data field
                        match event.event.as_str() {
                            "message_stop" => Ok(StreamEvent::MessageStop),
                            _ => Ok(StreamEvent::Ping),
                        }
                    } else {
                        // Parse JSON data
                        serde_json::from_str::<StreamEvent>(&event.data).map_err(|e| {
                            tracing::warn!(
                                "Failed to parse Anthropic stream event (type={}): {} | raw: {}",
                                event.event,
                                e,
                                &event.data[..event.data.len().min(500)]
                            );
                            AnthropicError::Json { source: e }
                        })
                    }
                }
                Err(e) => Err(AnthropicError::Stream {
                    message: e.to_string(),
                }),
            });

        Ok(Box::pin(event_stream))
    }

    /// Parse error response
    fn parse_error_response(&self, status: StatusCode, body: &str) -> AnthropicError {
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(error_json) => {
                if let Some(error_obj) = error_json["error"].as_object() {
                    let error_message = match error_obj["message"].as_str() {
                        Some(message) if !message.trim().is_empty() => message.to_string(),
                        _ => body.to_string(),
                    };

                    return AnthropicError::Api {
                        status,
                        message: error_message,
                    };
                }
            }
            Err(err) => {
                tracing::debug!(
                    status = %status,
                    error = %err,
                    "Failed to parse Anthropic error response as JSON"
                );
            }
        }

        AnthropicError::Api {
            status,
            message: body.to_string(),
        }
    }
}

// ============================================================
// Helper Functions: Apply Prompt Caching
// ============================================================

/// Add cache_control tags for models that support caching
///
/// Anthropic recommended caching strategy:
/// 1. Add caching to system prompt
/// 2. Add caching to the last two user messages
pub fn apply_prompt_caching(mut request: CreateMessageRequest) -> CreateMessageRequest {
    // 1. Add cache control to system prompt
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

    // 2. Find the last two user messages
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

    // 3. Add cache control to the last block of these two messages
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
                // Convert plain text to cached block
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
// LLMProvider Trait Implementation
// ============================================================

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    /// Non-streaming call - directly returns Anthropic native types
    async fn call(&self, request: CreateMessageRequest) -> LlmProviderResult<Message> {
        self.call_internal(request)
            .await
            .map_err(LlmProviderError::from)
    }

    /// Streaming call - directly returns Anthropic StreamEvent stream
    async fn call_stream(
        &self,
        request: CreateMessageRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<StreamEvent>> + Send>>> {
        // Call internal implementation
        let stream = self
            .call_stream_internal(request)
            .await
            .map_err(LlmProviderError::from)?;

        // Convert error type: AnthropicError -> LlmProviderError
        let converted_stream = stream.map(|result| result.map_err(LlmProviderError::from));

        Ok(Box::pin(converted_stream))
    }
}

// ============================================================
// Tests
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
            developer_context: None,
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
            thinking: None,
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
            developer_context: None,
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
            thinking: None,
        };

        let cached_request = apply_prompt_caching(request);

        // Check the 2nd and 3rd user messages (indices 2 and 4)
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
            oauth_config: None,
        };

        let provider = AnthropicProvider::new(config);
        let headers = provider
            .build_headers()
            .expect("test config should build valid headers");

        assert_eq!(headers.get("x-api-key").unwrap(), "test-key");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }
}
