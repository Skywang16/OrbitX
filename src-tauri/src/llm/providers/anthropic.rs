use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    error::{AnthropicError, LlmProviderError, LlmProviderResult},
    providers::base::LLMProvider,
    types::{
        LLMMessage, LLMMessageContent, LLMMessagePart, LLMProviderConfig, LLMRequest, LLMResponse,
        LLMStreamChunk, LLMTool, LLMToolCall, LLMUsage,
    },
};

/// Anthropic提供者
pub struct AnthropicProvider {
    client: Client,
    config: LLMProviderConfig,
}

type AnthropicResult<T> = Result<T, AnthropicError>;

impl AnthropicProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn get_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1");
        format!("{}/messages", base)
    }

    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), self.config.api_key.clone());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers
    }

    fn convert_messages(&self, messages: &[LLMMessage]) -> (Option<String>, Value) {
        let mut system_prompt = None;
        let mut converted_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                if let LLMMessageContent::Text(text) = &msg.content {
                    system_prompt = Some(text.clone());
                }
            } else {
                let content_parts: Vec<Value> = match &msg.content {
                    LLMMessageContent::Text(text) => vec![json!({ "type": "text", "text": text })],
                    LLMMessageContent::Parts(parts) => parts
                        .iter()
                        .filter_map(|part| match part {
                            LLMMessagePart::Text { text } => {
                                Some(json!({ "type": "text", "text": text }))
                            }
                            LLMMessagePart::ToolCall {
                                tool_call_id,
                                tool_name,
                                args,
                            } => Some(json!({
                                "type": "tool_use",
                                "id": tool_call_id,
                                "name": tool_name,
                                "input": args,
                            })),
                            LLMMessagePart::ToolResult {
                                tool_call_id,
                                result,
                                ..
                            } => Some(json!({
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": result.to_string(),
                            })),
                            LLMMessagePart::File { mime_type, data } => {
                                if mime_type.starts_with("image/") {
                                    Some(json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": mime_type,
                                            "data": data,
                                        }
                                    }))
                                } else {
                                    None
                                }
                            }
                        })
                        .collect(),
                };

                converted_messages.push(json!({
                    "role": msg.role,
                    "content": content_parts
                }));
            }
        }
        (system_prompt, json!(converted_messages))
    }

    fn build_body(&self, request: &LLMRequest) -> Value {
        let (system_prompt, messages) = self.convert_messages(&request.messages);

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": request.stream,
        });

        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = self.convert_tools(tools);
            }
        }

        body
    }

    fn convert_tools(&self, tools: &[LLMTool]) -> Value {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters,
                })
            })
            .collect()
    }

    fn parse_response(&self, response_json: &Value) -> AnthropicResult<LLMResponse> {
        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        if let Some(content_arr) = response_json["content"].as_array() {
            for item in content_arr {
                match item["type"].as_str() {
                    Some("text") => {
                        content_text.push_str(item["text"].as_str().unwrap_or(""));
                    }
                    Some("tool_use") => {
                        if let (Some(id), Some(name), Some(input)) = (
                            item["id"].as_str(),
                            item["name"].as_str(),
                            item["input"].as_object(),
                        ) {
                            tool_calls.push(LLMToolCall {
                                id: id.to_string(),
                                name: name.to_string(),
                                arguments: json!(input),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        let finish_reason = match response_json["stop_reason"].as_str() {
            Some("tool_use") => "tool_calls".to_string(),
            Some(reason) => reason.to_string(),
            None => "unknown".to_string(),
        };

        let usage = self.extract_usage(response_json);

        Ok(LLMResponse {
            content: content_text,
            finish_reason,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            usage,
        })
    }

    fn extract_usage(&self, response_json: &Value) -> Option<LLMUsage> {
        response_json["usage"].as_object().map(|usage_obj| {
            let prompt_tokens = usage_obj["input_tokens"].as_u64().unwrap_or(0) as u32;
            let completion_tokens = usage_obj["output_tokens"].as_u64().unwrap_or(0) as u32;
            LLMUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }
        })
    }

    fn handle_error_response(&self, status: StatusCode, body: &str) -> AnthropicError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json["error"].as_object() {
                let error_type = error_obj["type"].as_str().unwrap_or("unknown_error");
                let error_message = error_obj["message"].as_str().unwrap_or("Unknown error");
                return AnthropicError::Api {
                    status,
                    message: format!("{}: {}", error_type, error_message),
                };
            }
        }
        AnthropicError::Api {
            status,
            message: format!("Unexpected response: {}", body),
        }
    }

    /// 解析content_block_start事件，处理工具调用
    fn parse_content_block_start(event_json: &Value) -> Option<LlmProviderResult<LLMStreamChunk>> {
        let content_block = &event_json["content_block"];

        if content_block["type"] == "tool_use" {
            // 提取工具调用信息
            let id = content_block["id"].as_str().unwrap_or("").to_string();
            let name = content_block["name"].as_str().unwrap_or("").to_string();

            if !name.is_empty() {
                let tool_call = LLMToolCall {
                    id,
                    name,
                    arguments: serde_json::json!({}),
                };

                Some(Ok(LLMStreamChunk::Delta {
                    content: None,
                    tool_calls: Some(vec![tool_call]),
                }))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_stream_chunk(data: &str) -> Option<LlmProviderResult<LLMStreamChunk>> {
        let event_json: Value = match serde_json::from_str(data) {
            Ok(json) => json,
            Err(_) => return None,
        };

        let event_type = event_json["type"].as_str().unwrap_or("");

        match event_type {
            "content_block_delta" => {
                if event_json["delta"]["type"] == "text_delta" {
                    let content = event_json["delta"]["text"].as_str().map(|s| s.to_string());
                    Some(Ok(LLMStreamChunk::Delta {
                        content,
                        tool_calls: None,
                    }))
                } else if event_json["delta"]["type"] == "input_json_delta" {
                    None
                } else {
                    None
                }
            }
            "content_block_start" => Self::parse_content_block_start(&event_json),
            "content_block_stop" => None,
            "message_stop" => {
                let stop_reason = event_json["stop_reason"].as_str().unwrap_or("stop");
                let finish_reason = match stop_reason {
                    "tool_use" => "tool_calls".to_string(),
                    other => other.to_string(),
                };
                Some(Ok(LLMStreamChunk::Finish {
                    finish_reason,
                    usage: None,
                }))
            }
            "error" => {
                let error_message = event_json["error"]["message"]
                    .as_str()
                    .unwrap_or("Unknown stream error")
                    .to_string();
                Some(Err(LlmProviderError::Anthropic(AnthropicError::Stream {
                    message: error_message,
                })))
            }
            _ => None,
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn call(&self, request: LLMRequest) -> LlmProviderResult<LLMResponse> {
        let url = self.get_endpoint();
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|source| LlmProviderError::Anthropic(AnthropicError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let error = self.handle_error_response(status, &text);
            return Err(LlmProviderError::from(error));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|source| LlmProviderError::Anthropic(AnthropicError::Http { source }))?;
        self.parse_response(&response_json)
            .map_err(LlmProviderError::from)
    }

    async fn call_stream(
        &self,
        mut request: LLMRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<LLMStreamChunk>> + Send>>>
    {
        request.stream = true;
        let url = self.get_endpoint();
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|source| LlmProviderError::Anthropic(AnthropicError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let error = self.handle_error_response(status, &text);
            return Err(LlmProviderError::from(error));
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .filter_map(|event_result| {
                futures::future::ready(match event_result {
                    Ok(event) => Self::parse_stream_chunk(&event.data),
                    Err(e) => Some(Err(LlmProviderError::Anthropic(AnthropicError::Stream {
                        message: format!("Network error: {}", e),
                    }))),
                })
            });

        Ok(Box::pin(stream))
    }
}
