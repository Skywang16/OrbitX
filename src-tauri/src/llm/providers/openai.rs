use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    providers::base::LLMProvider,
    types::{
        LLMError, LLMMessage, LLMMessageContent, LLMMessagePart, LLMProviderConfig, LLMRequest,
        LLMResponse, LLMResult, LLMStreamChunk, LLMTool, LLMToolCall, LLMUsage,
    },
};

/// OpenAI Provider
///
/// 实现了与 OpenAI API (以及兼容 OpenAI API 的服务) 的所有交互逻辑
pub struct OpenAIProvider {
    client: Client,
    config: LLMProviderConfig,
}

impl OpenAIProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    // --- Private Helper Methods ---

    /// 构建请求体
    fn build_body(&self, request: &LLMRequest) -> Value {
        let mut body = json!({
            "model": request.model,
            "messages": self.convert_messages(&request.messages),
            "stream": request.stream,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = self.convert_tools(tools);
                if let Some(tool_choice) = &request.tool_choice {
                    body["tool_choice"] = json!(tool_choice);
                }
            }
        }

        body
    }

    /// 获取 API 端点
    fn get_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{}/chat/completions", base)
    }

    /// 获取请求头
    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.config.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }

    /// 转换统一的工具定义为 OpenAI 特定格式
    fn convert_tools(&self, tools: &[LLMTool]) -> Value {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                })
            })
            .collect();
        json!(openai_tools)
    }

    /// 转换统一的消息为 OpenAI 特定格式
    fn convert_messages(&self, messages: &[LLMMessage]) -> Value {
        let openai_messages: Vec<Value> = messages
            .iter()
            .map(|msg| {
                // Assistant 的 tool_calls 需要特殊处理
                if msg.role == "assistant" {
                    if let LLMMessageContent::Parts(parts) = &msg.content {
                        let tool_calls: Vec<Value> = parts
                            .iter()
                            .filter_map(|part| {
                                if let LLMMessagePart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    args,
                                } = part
                                {
                                    Some(json!({
                                        "id": tool_call_id,
                                        "type": "function",
                                        "function": {
                                            "name": tool_name,
                                            "arguments": args.to_string()
                                        }
                                    }))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !tool_calls.is_empty() {
                            return json!({
                                "role": "assistant",
                                "content": null,
                                "tool_calls": tool_calls
                            });
                        }
                    }
                }

                // Tool 角色的消息需要特殊处理
                if msg.role == "tool" {
                    if let LLMMessageContent::Parts(parts) = &msg.content {
                        if let Some(LLMMessagePart::ToolResult {
                            tool_call_id,
                            result,
                            ..
                        }) = parts.first()
                        {
                            return json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "content": result.to_string()
                            });
                        }
                    }
                }

                // 其他通用消息
                let content = match &msg.content {
                    LLMMessageContent::Text(text) => json!(text),
                    LLMMessageContent::Parts(parts) => {
                        let content_parts: Vec<Value> = parts
                            .iter()
                            .map(|part| match part {
                                LLMMessagePart::Text { text } => json!({ "type": "text", "text": text }),
                                LLMMessagePart::File { mime_type, data } => {
                                    json!({
                                        "type": "image_url",
                                        "image_url": { "url": format!("data:{};base64,{}", mime_type, data) }
                                    })
                                }
                                // ToolCall 和 ToolResult 已被特殊处理，这里可以忽略或作为文本回退
                                _ => json!({ "type": "text", "text": "Unsupported content part" }),
                            })
                            .collect();
                        json!(content_parts)
                    }
                };

                json!({ "role": msg.role, "content": content })
            })
            .collect();

        json!(openai_messages)
    }

    /// 从提供商响应中解析 LLMResponse
    fn parse_response(&self, response_json: &Value) -> LLMResult<LLMResponse> {
        let choice = response_json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| LLMError::InvalidResponse("Missing 'choices' array".to_string()))?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = choice["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        let tool_calls = self.extract_tool_calls(choice)?;
        let usage = self.extract_usage(response_json);

        Ok(LLMResponse {
            content,
            finish_reason,
            tool_calls,
            usage,
        })
    }

    /// 从响应中提取工具调用
    fn extract_tool_calls(&self, choice: &Value) -> LLMResult<Option<Vec<LLMToolCall>>> {
        if let Some(tool_calls_json) = choice["message"]["tool_calls"].as_array() {
            let extracted_calls: LLMResult<Vec<LLMToolCall>> = tool_calls_json
                .iter()
                .map(|tc| {
                    let id = tc["id"]
                        .as_str()
                        .ok_or_else(|| {
                            LLMError::InvalidResponse("Missing 'id' in tool_call".to_string())
                        })?
                        .to_string();
                    let function = &tc["function"];
                    let name = function["name"]
                        .as_str()
                        .ok_or_else(|| {
                            LLMError::InvalidResponse(
                                "Missing 'name' in tool_call function".to_string(),
                            )
                        })?
                        .to_string();

                    let arguments_str = function["arguments"].as_str().unwrap_or("{}");
                    let arguments: Value = serde_json::from_str(arguments_str).map_err(|e| {
                        LLMError::InvalidResponse(format!(
                            "Failed to parse tool_call arguments: {}",
                            e
                        ))
                    })?;

                    Ok(LLMToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect();

            return Ok(Some(extracted_calls?));
        }
        Ok(None)
    }

    /// 从响应中提取使用统计
    fn extract_usage(&self, response_json: &Value) -> Option<LLMUsage> {
        response_json["usage"]
            .as_object()
            .map(|usage_obj| LLMUsage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
    }

    /// 处理 API 错误响应
    fn handle_error_response(&self, status: u16, body: &str) -> LLMError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json["error"].as_object() {
                let error_type = error_obj["type"].as_str().unwrap_or("unknown");
                let error_message = error_obj["message"].as_str().unwrap_or("Unknown error");

                return match error_type {
                    "insufficient_quota" => {
                        LLMError::Provider(format!("Quota exceeded: {}", error_message))
                    }
                    "invalid_request_error" => {
                        LLMError::Config(format!("Invalid request: {}", error_message))
                    }
                    "authentication_error" => {
                        LLMError::Config(format!("Authentication failed: {}", error_message))
                    }
                    _ => LLMError::Provider(format!("OpenAI API error: {}", error_message)),
                };
            }
        }
        LLMError::Provider(format!("OpenAI API error {}: {}", status, body))
    }

    /// 解析 SSE 流中的单个数据块 (关联函数，不依赖 self)
    fn parse_stream_chunk(data: &str) -> LLMResult<LLMStreamChunk> {
        if data == "[DONE]" {
            return Ok(LLMStreamChunk::Finish {
                finish_reason: "stop".to_string(),
                usage: None, // Usage is often sent in a separate event or not at all in streams
            });
        }

        let json_data: Value = serde_json::from_str(data).map_err(|e| {
            LLMError::InvalidResponse(format!("Failed to parse stream data: {}", e))
        })?;

        let choice = json_data["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                LLMError::InvalidResponse("Missing 'choices' in stream chunk".to_string())
            })?;

        if let Some(finish_reason) = choice["finish_reason"].as_str() {
            return Ok(LLMStreamChunk::Finish {
                finish_reason: finish_reason.to_string(),
                // 在流的末尾，OpenAI 可能会附带 usage 统计
                usage: Self::extract_usage_static(&json_data),
            });
        }

        if let Some(delta) = choice.get("delta") {
            let content = delta["content"].as_str().map(|s| s.to_string());
            // TODO: 流式工具调用解析
            let tool_calls = None;
            return Ok(LLMStreamChunk::Delta {
                content,
                tool_calls,
            });
        }

        Err(LLMError::InvalidResponse(
            "Unknown stream chunk format".to_string(),
        ))
    }

    // extract_usage 的静态版本
    fn extract_usage_static(response_json: &Value) -> Option<LLMUsage> {
        response_json["usage"]
            .as_object()
            .map(|usage_obj| LLMUsage {
                prompt_tokens: usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// 非流式调用
    async fn call(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let url = self.get_endpoint();
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.map_err(LLMError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.handle_error_response(status.as_u16(), &error_text));
        }

        let response_json: Value = response.json().await.map_err(LLMError::Http)?;
        self.parse_response(&response_json)
    }

    /// 流式调用
    async fn call_stream(
        &self,
        mut request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<LLMStreamChunk>> + Send>>> {
        request.stream = true;
        let url = self.get_endpoint();
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.map_err(LLMError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(self.handle_error_response(status.as_u16(), &error_text));
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event_result| {
                match event_result {
                    Ok(event) => {
                        // 解析SSE事件数据
                        Self::parse_stream_chunk(&event.data)
                    }
                    Err(e) => Err(LLMError::Network(e.to_string())),
                }
            })
            .filter(|result| {
                // 过滤掉空的delta消息
                futures::future::ready(match result {
                    Ok(LLMStreamChunk::Delta {
                        content: None,
                        tool_calls: None,
                    }) => false,
                    _ => true,
                })
            });

        Ok(Box::pin(stream))
    }
}
