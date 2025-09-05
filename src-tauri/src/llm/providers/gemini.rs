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

/// Google Gemini Provider
///
/// 实现了与 Google Gemini API 的所有交互逻辑
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

    // --- Private Helper Methods ---

    fn get_endpoint(&self, request: &LLMRequest) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta/models");
        let stream_suffix = if request.stream {
            ":streamGenerateContent"
        } else {
            ":generateContent"
        };
        format!(
            "{}/{}{}?key={}",
            base, request.model, stream_suffix, self.config.api_key
        )
    }

    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }

    fn convert_contents(&self, messages: &[LLMMessage]) -> (Option<Value>, Value) {
        let mut system_instruction = None;
        let mut contents = Vec::new();
        let mut current_role: Option<String> = None;
        let mut current_parts = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                if let LLMMessageContent::Text(text) = &msg.content {
                    system_instruction = Some(json!({ "parts": [{ "text": text }] }));
                }
                continue;
            }

            let role = if msg.role == "assistant" {
                "model".to_string()
            } else {
                msg.role.clone()
            };

            if current_role.is_some() && current_role.as_deref() != Some(&role) {
                contents.push(json!({
                    "role": current_role.unwrap(),
                    "parts": current_parts,
                }));
                current_parts = Vec::new();
            }
            current_role = Some(role);

            let mut parts = match &msg.content {
                LLMMessageContent::Text(text) => vec![json!({ "text": text })],
                LLMMessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|part| match part {
                        LLMMessagePart::Text { text } => Some(json!({ "text": text })),
                        LLMMessagePart::File { mime_type, data } => Some(json!({
                            "inline_data": { "mime_type": mime_type, "data": data }
                        })),
                        LLMMessagePart::ToolResult {
                            tool_name, result, ..
                        } => Some(json!({
                            "functionResponse": {
                                "name": tool_name,
                                "response": { "result": result }
                            }
                        })),
                        _ => None,
                    })
                    .collect(),
            };
            current_parts.append(&mut parts);
        }

        if let Some(role) = current_role {
            contents.push(json!({ "role": role, "parts": current_parts }));
        }

        (system_instruction, json!(contents))
    }

    fn build_body(&self, request: &LLMRequest) -> Value {
        let (system_instruction, contents) = self.convert_contents(&request.messages);

        let mut body = json!({ "contents": contents });

        if let Some(system) = system_instruction {
            body["system_instruction"] = system;
        }

        let mut generation_config = json!({});
        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        body["generationConfig"] = generation_config;

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = json!([{ "function_declarations": self.convert_tools(tools) }]);
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
                    "parameters": tool.parameters,
                })
            })
            .collect()
    }

    fn parse_response(response_json: &Value) -> LLMResult<LLMResponse> {
        let candidate = response_json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| LLMError::InvalidResponse("Missing 'candidates' in response".into()))?;

        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for part in parts {
                if let Some(text) = part["text"].as_str() {
                    content_text.push_str(text);
                }
                if let Some(fc) = part["functionCall"].as_object() {
                    println!("🔧 Debug: Found functionCall in Gemini response");
                    if let (Some(name), Some(args)) = (fc["name"].as_str(), fc["args"].as_object())
                    {
                        let tool_call = LLMToolCall {
                            // Gemini doesn't provide a unique ID, so we generate one.
                            id: format!("tool-call-{}", uuid::Uuid::new_v4()),
                            name: name.to_string(),
                            arguments: json!(args),
                        };
                        println!(
                            "🔧 Debug: Creating Gemini tool call - id: {}, name: {}",
                            tool_call.id, tool_call.name
                        );
                        tool_calls.push(tool_call);
                    }
                }
            }
        }

        let finish_reason = match candidate["finishReason"].as_str() {
            Some("STOP") => {
                // 如果有工具调用，则finish_reason应该是tool_calls
                if !tool_calls.is_empty() {
                    "tool_calls".to_string()
                } else {
                    "stop".to_string()
                }
            }
            Some(reason) => reason.to_lowercase(),
            None => "unknown".to_string(),
        };

        let usage = Self::extract_usage(response_json);

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

    fn extract_usage(response_json: &Value) -> Option<LLMUsage> {
        response_json["usageMetadata"].as_object().map(|usage_obj| {
            let prompt_tokens = usage_obj["promptTokenCount"].as_u64().unwrap_or(0) as u32;
            let completion_tokens = usage_obj["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
            LLMUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: usage_obj["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            }
        })
    }

    fn handle_error_response(&self, status: u16, body: &str) -> LLMError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json["error"].as_object() {
                let message = error_obj["message"]
                    .as_str()
                    .unwrap_or("Unknown Gemini error");
                return LLMError::Provider(message.to_string());
            }
        }
        LLMError::Provider(format!("Gemini API error {}: {}", status, body))
    }

    fn parse_stream_chunk(data: &str) -> LLMResult<LLMStreamChunk> {
        let json_data: Value = serde_json::from_str(data).map_err(|e| {
            LLMError::InvalidResponse(format!("Failed to parse stream data: {}", e))
        })?;

        // Stream response is an array of candidates
        if let Ok(response) = Self::parse_response(&json_data) {
            let chunk = LLMStreamChunk::Delta {
                content: Some(response.content),
                tool_calls: response.tool_calls,
            };

            if response.finish_reason != "unknown"
                && response.finish_reason != "FINISH_REASON_UNSPECIFIED"
            {
                return Ok(LLMStreamChunk::Finish {
                    finish_reason: response.finish_reason,
                    usage: response.usage,
                });
            }
            return Ok(chunk);
        }

        Err(LLMError::InvalidResponse("Unknown stream format".into()))
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn call(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let url = self.get_endpoint(&request);
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.map_err(LLMError::Http)?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(self.handle_error_response(status.as_u16(), &text));
        }

        let response_json: Value = response.json().await.map_err(LLMError::Http)?;
        Self::parse_response(&response_json)
    }

    async fn call_stream(
        &self,
        mut request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<LLMStreamChunk>> + Send>>> {
        request.stream = true;
        let url = self.get_endpoint(&request);
        let headers = self.get_headers();
        let body = self.build_body(&request);

        let mut req_builder = self.client.post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.map_err(LLMError::Http)?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(self.handle_error_response(status.as_u16(), &text));
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .filter_map(|event_result| {
                futures::future::ready(match event_result {
                    Ok(event) => {
                        // Gemini可能不使用标准SSE格式，直接解析数据
                        if event.data.trim().is_empty() {
                            None
                        } else {
                            Some(Self::parse_stream_chunk(&event.data))
                        }
                    }
                    Err(e) => Some(Err(LLMError::Network(e.to_string()))),
                })
            });

        Ok(Box::pin(stream))
    }
}
