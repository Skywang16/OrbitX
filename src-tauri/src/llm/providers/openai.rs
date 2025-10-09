use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult, OpenAiError},
    providers::base::LLMProvider,
    types::{
        EmbeddingData, EmbeddingRequest, EmbeddingResponse, LLMMessage, LLMMessageContent,
        LLMMessagePart, LLMProviderConfig, LLMRequest, LLMResponse, LLMStreamChunk, LLMTool,
        LLMToolCall, LLMUsage,
    },
};

/// OpenAI提供者
pub struct OpenAIProvider {
    client: Client,
    config: LLMProviderConfig,
}

type OpenAiResult<T> = Result<T, OpenAiError>;

impl OpenAIProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

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

    /// 获取 Embedding API 端点
    fn get_embedding_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{}/embeddings", base)
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
    fn parse_response(&self, response_json: &Value) -> OpenAiResult<LLMResponse> {
        let choice = response_json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or(OpenAiError::MissingField { field: "choices" })?;

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
    fn extract_tool_calls(&self, choice: &Value) -> OpenAiResult<Option<Vec<LLMToolCall>>> {
        if let Some(tool_calls_json) = choice["message"]["tool_calls"].as_array() {
            let extracted_calls: OpenAiResult<Vec<LLMToolCall>> = tool_calls_json
                .iter()
                .map(|tc| {
                    let id = tc["id"]
                        .as_str()
                        .ok_or(OpenAiError::MissingField { field: "tool_calls[].id" })?
                        .to_string();
                    let function = &tc["function"];
                    let name = function["name"]
                        .as_str()
                        .ok_or(OpenAiError::MissingField {
                            field: "tool_calls[].function.name",
                        })?
                        .to_string();

                    let arguments_str = function["arguments"].as_str().unwrap_or("{}");
                    let arguments: Value = serde_json::from_str(arguments_str)
                        .map_err(|source| OpenAiError::ToolCallArguments { source })?;

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
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            })
    }

    /// 处理 API 错误响应
    fn handle_error_response(&self, status: StatusCode, body: &str) -> OpenAiError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json["error"].as_object() {
                let error_type = error_obj["type"].as_str().unwrap_or("unknown");
                let error_message = error_obj["message"].as_str().unwrap_or("Unknown error");

                let message = match error_type {
                    "insufficient_quota" => format!("Quota exceeded: {}", error_message),
                    "invalid_request_error" => format!("Request error: {}", error_message),
                    "authentication_error" => format!("Authentication failed: {}", error_message),
                    _ => error_message.to_string(),
                };

                return OpenAiError::Api { status, message };
            }
        }
        OpenAiError::Api {
            status,
            message: format!("Unexpected response: {}", body),
        }
    }

    /// 解析流式响应中的工具调用增量
    fn parse_delta_tool_calls(delta: &Value) -> OpenAiResult<Option<Vec<LLMToolCall>>> {
        if let Some(tool_calls_array) = delta["tool_calls"].as_array() {
            let mut parsed_calls = Vec::new();

            for tool_call_delta in tool_calls_array {
                let index = tool_call_delta["index"].as_u64().unwrap_or(0) as usize;

                // 工具调用ID（可能在第一个delta中出现）
                let id = tool_call_delta["id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("tool_call_{}", index));

                // 解析function信息
                if let Some(function_delta) = tool_call_delta.get("function") {
                    let name = function_delta["name"]
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    // 参数可能是增量式的，这里我们收集所有可用的参数
                    let arguments_str = function_delta["arguments"].as_str().unwrap_or("{}");

                    // 尝试解析参数，如果解析失败则使用空对象
                    let arguments = if arguments_str.is_empty() {
                        serde_json::Value::Object(serde_json::Map::new())
                    } else {
                        serde_json::from_str(arguments_str).unwrap_or_else(|_| {
                            serde_json::json!({
                                "_streaming_args": arguments_str,
                                "_is_streaming": true
                            })
                        })
                    };

                    // 只有当我们有名称时才创建工具调用
                    if !name.is_empty() {
                        parsed_calls.push(LLMToolCall {
                            id,
                            name,
                            arguments,
                        });
                    }
                }
            }

            if !parsed_calls.is_empty() {
                return Ok(Some(parsed_calls));
            }
        }

        Ok(None)
    }

    /// 解析 SSE 流中的单个数据块 (关联函数，不依赖 self)
    fn parse_stream_chunk(data: &str) -> OpenAiResult<LLMStreamChunk> {
        if data == "[DONE]" {
            return Ok(LLMStreamChunk::Finish {
                finish_reason: "stop".to_string(),
                usage: None, // Usage is often sent in a separate event or not at all in streams
            });
        }

        // 跳过空数据或无效数据
        if data.trim().is_empty() {
            return Err(OpenAiError::Stream {
                message: "Empty stream payload".to_string(),
            });
        }

        let json_data: Value =
            serde_json::from_str(data).map_err(|source| OpenAiError::Json { source })?;

        let choice = json_data["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or(OpenAiError::MissingField { field: "choices" })?;

        if let Some(finish_reason) = choice["finish_reason"].as_str() {
            // 当finish_reason是"tool_calls"时，检查是否有工具调用需要处理
            if finish_reason == "tool_calls" {
                if let Some(delta) = choice.get("delta") {
                    let tool_calls = Self::parse_delta_tool_calls(delta)?;
                    if tool_calls.is_some() {
                        return Ok(LLMStreamChunk::Delta {
                            content: None,
                            tool_calls,
                        });
                    }
                }
            }

            return Ok(LLMStreamChunk::Finish {
                finish_reason: finish_reason.to_string(),
                // 在流的末尾，OpenAI 可能会附带 usage 统计
                usage: Self::extract_usage_static(&json_data),
            });
        }

        if let Some(delta) = choice.get("delta") {
            let content = delta["content"].as_str().map(|s| s.to_string());

            // 解析流式工具调用
            let tool_calls = Self::parse_delta_tool_calls(delta)?;

            // 只有当有实际内容或工具调用时才返回delta
            if (content.is_some() && !content.as_ref().unwrap().is_empty()) || tool_calls.is_some()
            {
                return Ok(LLMStreamChunk::Delta {
                    content,
                    tool_calls,
                });
            } else {
                // 跳过空的delta
                return Err(OpenAiError::Stream {
                    message: "Empty delta chunk".to_string(),
                });
            }
        }

        Err(OpenAiError::Stream {
            message: "Unknown stream chunk format".to_string(),
        })
    }

    // extract_usage 的静态版本
    fn extract_usage_static(response_json: &Value) -> Option<LLMUsage> {
        response_json["usage"]
            .as_object()
            .map(|usage_obj| LLMUsage {
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            })
    }

    /// 解析embedding响应
    fn parse_embedding_response(&self, response_json: &Value) -> OpenAiResult<EmbeddingResponse> {
        let data_array = response_json["data"]
            .as_array()
            .ok_or(OpenAiError::EmbeddingField { field: "data" })?;

        let mut embedding_data = Vec::new();
        for (i, item) in data_array.iter().enumerate() {
            let embedding_vec = item["embedding"]
                .as_array()
                .ok_or(OpenAiError::EmbeddingField { field: "data[].embedding" })?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect::<Vec<f32>>();

            embedding_data.push(EmbeddingData {
                embedding: embedding_vec,
                index: item["index"].as_u64().unwrap_or(i as u64) as usize,
                object: item["object"].as_str().unwrap_or("embedding").to_string(),
            });
        }

        let model = response_json["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let usage = Self::extract_usage_static(response_json);

        Ok(EmbeddingResponse {
            data: embedding_data,
            model,
            usage,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// 非流式调用
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
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error = self.handle_error_response(status, &error_text);
            return Err(LlmProviderError::from(error));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        self.parse_response(&response_json)
            .map_err(LlmProviderError::from)
    }

    /// 流式调用
    async fn call_stream(
        &self,
        mut request: LLMRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<LLMStreamChunk>> + Send>>> {
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
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error = self.handle_error_response(status, &error_text);
            return Err(LlmProviderError::from(error));
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .filter_map(|event_result| {
                futures::future::ready(match event_result {
                    Ok(event) => {
                        // 解析SSE事件数据
                        match Self::parse_stream_chunk(&event.data) {
                            Ok(chunk) => Some(Ok(chunk)),
                            Err(_) => None, // 静默跳过解析错误（通常是空内容）
                        }
                    }
                    Err(e) => Some(Err(LlmProviderError::OpenAi(OpenAiError::Stream {
                        message: format!("Network error: {e}"),
                    }))),
                })
            });

        Ok(Box::pin(stream))
    }

    /// Embedding调用实现
    async fn create_embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> LlmProviderResult<EmbeddingResponse> {
        let url = self.get_embedding_endpoint();
        let headers = self.get_headers();

        // 构建embedding请求体
        let mut body = json!({
            "model": request.model,
            "input": request.input
        });

        if let Some(encoding_format) = &request.encoding_format {
            body["encoding_format"] = json!(encoding_format);
        }

        if let Some(dimensions) = request.dimensions {
            body["dimensions"] = json!(dimensions);
        }

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        tracing::debug!("Making embedding API call to: {}", url);

        let response = req_builder
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error = self.handle_error_response(status, &error_text);
            return Err(LlmProviderError::from(error));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        self.parse_embedding_response(&response_json)
            .map_err(LlmProviderError::from)
    }
}
