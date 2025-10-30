use async_trait::async_trait;
use eventsource_stream::Eventsource;
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::Stream;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult, OpenAiError},
    providers::base::LLMProvider,
    types::{EmbeddingData, EmbeddingRequest, EmbeddingResponse, LLMProviderConfig, LLMUsage},
};

/// 全局共享的HTTP客户端，优化连接复用
static SHARED_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create shared HTTP client")
});

/// OpenAI Provider (messages unsupported in zero-abstraction mode)
/// 使用全局共享HTTP客户端优化性能
pub struct OpenAIProvider {
    config: LLMProviderConfig,
}

type OpenAiResult<T> = Result<T, OpenAiError>;

fn build_openai_chat_body(
    req: &crate::llm::anthropic_types::CreateMessageRequest,
    stream: bool,
) -> Value {
    use crate::llm::anthropic_types::SystemPrompt;
    let mut chat_messages: Vec<Value> = Vec::new();
    if let Some(system) = &req.system {
        let sys_text = match system {
            SystemPrompt::Text(t) => t.clone(),
            SystemPrompt::Blocks(_blocks) => {
                // 仅提取文本，blocks暂不支持，返回空字符串
                String::new()
            }
        };
        if !sys_text.is_empty() {
            chat_messages.push(json!({"role":"system","content":sys_text}));
        }
    }
    let converted = crate::llm::transform::openai::convert_to_openai_messages(&req.messages);
    chat_messages.extend(converted);

    let tools_val = req.tools.as_ref().map(|tools| {
        Value::Array(tools.iter().map(|t| json!({
            "type": "function",
            "function": {"name": t.name, "description": t.description, "parameters": t.input_schema}
        })).collect())
    });

    let mut body = json!({
        "model": req.model,
        "messages": chat_messages,
        "stream": stream
    });
    if let Some(temp) = req.temperature {
        body["temperature"] = json!(temp);
    }
    body["max_tokens"] = json!(req.max_tokens);
    if let Some(tv) = tools_val {
        body["tools"] = tv;
        body["tool_choice"] = json!("auto");
    }

    body
}

impl OpenAIProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self { config }
    }

    /// 获取共享HTTP客户端
    fn client(&self) -> &'static Client {
        &SHARED_HTTP_CLIENT
    }

    /// 获取 Chat Completions 端点
    fn get_chat_endpoint(&self) -> String {
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

    /// 处理 API 错误响应
    fn handle_error_response(&self, status: StatusCode, body: &str) -> OpenAiError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json.get("error").and_then(|v| v.as_object()) {
                let error_type = error_obj
                    .get("type")
                    .or_else(|| error_obj.get("code"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let error_message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");

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
            message: format!("意外的响应: {}", body),
        }
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
                .ok_or(OpenAiError::EmbeddingField {
                    field: "data[].embedding",
                })?
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
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// 非流式调用（Anthropic 原生接口）
    async fn call(
        &self,
        request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<crate::llm::anthropic_types::Message> {
        use crate::llm::anthropic_types::{ContentBlock, Message, MessageRole, Usage};

        let url = self.get_chat_endpoint();
        let headers = self.get_headers();
        let body = build_openai_chat_body(&request, false);

        let mut req = self.client().post(&url).json(&body);
        for (k, v) in headers {
            req = req.header(&k, &v);
        }

        let resp = req
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmProviderError::from(
                self.handle_error_response(status, &txt),
            ));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        // 解析文本
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let usage_obj = json.get("usage");
        let usage = usage_obj
            .and_then(|u| {
                Some(Usage {
                    input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                        as u32,
                    output_tokens: u
                        .get("completion_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                })
            })
            .unwrap_or(Usage {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            });

        let message = Message {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            message_type: "message".to_string(),
            role: MessageRole::Assistant,
            content: vec![ContentBlock::Text {
                text: content,
                cache_control: None,
            }],
            model: request.model.clone(),
            stop_reason: None,
            stop_sequence: None,
            usage,
        };
        Ok(message)
    }

    /// 流式调用（Anthropic 原生接口）
    async fn call_stream(
        &self,
        request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<
        Pin<
            Box<
                dyn Stream<Item = LlmProviderResult<crate::llm::anthropic_types::StreamEvent>>
                    + Send,
            >,
        >,
    > {
        use crate::llm::anthropic_types::{
            ContentBlockStart, ContentDelta, MessageDeltaData, MessageRole, MessageStartData,
            StopReason, StreamEvent, Usage,
        };

        let url = self.get_chat_endpoint();
        let headers = self.get_headers();
        let body = build_openai_chat_body(&request, true);

        // tracing::debug!(
        // "OpenAI stream request body: {}",
        // serde_json::to_string_pretty(&body).unwrap_or_else(|_| format!("{:?}", body))
        // );

        let mut req = self.client().post(&url).json(&body);
        for (k, v) in headers {
            req = req.header(&k, &v);
        }

        let resp = req
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = resp.status();
        if !status.is_success() {
            let txt = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmProviderError::from(
                self.handle_error_response(status, &txt),
            ));
        }

        use futures::stream;
        use futures::StreamExt as FuturesStreamExt;

        // 状态机：记录是否已发送关键事件
        struct StreamState {
            message_started: bool,
            content_block_started: bool,
            pending_events: VecDeque<StreamEvent>,
            tool_use_started: HashSet<usize>,
        }

        let model = request.model.clone();
        let raw_stream = resp.bytes_stream().eventsource();

        // 使用 unfold 来维护状态
        let event_stream = stream::unfold(
            (
                raw_stream,
                StreamState {
                    message_started: false,
                    content_block_started: false,
                    pending_events: VecDeque::new(),
                    tool_use_started: HashSet::new(),
                },
            ),
            move |(mut stream, mut state)| {
                let model = model.clone();
                async move {
                    loop {
                        // 优先输出已排队的事件
                        if let Some(evt) = state.pending_events.pop_front() {
                            return Some((Ok(evt), (stream, state)));
                        }
                        match FuturesStreamExt::next(&mut stream).await {
                            Some(Ok(event)) => {
                                // OpenAI 流结束标记
                                if event.data == "[DONE]" {
                                    tracing::debug!("OpenAI stream finished");
                                    // 结束前关闭未关闭的块
                                    if state.content_block_started {
                                        state.content_block_started = false;
                                        state
                                            .pending_events
                                            .push_back(StreamEvent::ContentBlockStop { index: 0 });
                                    }
                                    if !state.tool_use_started.is_empty() {
                                        let indices: Vec<usize> =
                                            state.tool_use_started.iter().copied().collect();
                                        for idx in indices {
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStop { index: idx },
                                            );
                                        }
                                        state.tool_use_started.clear();
                                    }
                                    state.pending_events.push_back(StreamEvent::MessageStop);
                                    if let Some(evt) = state.pending_events.pop_front() {
                                        return Some((Ok(evt), (stream, state)));
                                    } else {
                                        continue;
                                    }
                                }

                                // 解析 OpenAI 的流式响应
                                let value: Value = match serde_json::from_str(&event.data) {
                                    Ok(v) => v,
                                    Err(_) => continue, // 跳过无效数据
                                };

                                // 提取 choices[0]
                                let choice =
                                    match value["choices"].as_array().and_then(|arr| arr.first()) {
                                        Some(c) => c,
                                        None => continue,
                                    };

                                let delta = &choice["delta"];

                                // 第一个事件：MessageStart
                                if !state.message_started {
                                    state.message_started = true;
                                    state.pending_events.push_back(StreamEvent::MessageStart {
                                        message: MessageStartData {
                                            id: format!("msg_{}", uuid::Uuid::new_v4()),
                                            message_type: "message".to_string(),
                                            role: MessageRole::Assistant,
                                            model: model.clone(),
                                            usage: Usage {
                                                input_tokens: 0,
                                                output_tokens: 0,
                                                cache_creation_input_tokens: None,
                                                cache_read_input_tokens: None,
                                            },
                                        },
                                    });
                                }

                                // 第二个事件：ContentBlockStart（当第一次遇到 content 时）
                                if !state.content_block_started && delta.get("content").is_some() {
                                    state.content_block_started = true;
                                    state.pending_events.push_back(
                                        StreamEvent::ContentBlockStart {
                                            index: 0,
                                            content_block: ContentBlockStart::Text {
                                                text: String::new(),
                                            },
                                        },
                                    );
                                }

                                // ContentBlockDelta（content 增量）
                                if let Some(content) = delta["content"].as_str() {
                                    if !content.is_empty() {
                                        state.pending_events.push_back(
                                            StreamEvent::ContentBlockDelta {
                                                index: 0,
                                                delta: ContentDelta::TextDelta {
                                                    text: content.to_string(),
                                                },
                                            },
                                        );
                                    }
                                }

                                // 处理工具调用增量 delta.tool_calls
                                if let Some(tc_arr) =
                                    delta.get("tool_calls").and_then(|v| v.as_array())
                                {
                                    for tc in tc_arr {
                                        let raw_index =
                                            tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0)
                                                as usize;
                                        let event_index = raw_index + 1; // 将工具块索引与文本块(0)错开

                                        let func = tc.get("function");
                                        let name_opt = func
                                            .and_then(|f| f.get("name"))
                                            .and_then(|v| v.as_str());
                                        let args_opt = func
                                            .and_then(|f| f.get("arguments"))
                                            .and_then(|v| v.as_str());

                                        if !state.tool_use_started.contains(&event_index) {
                                            if let Some(name) = name_opt {
                                                let id = tc
                                                    .get("id")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| {
                                                        format!("call_{}", uuid::Uuid::new_v4())
                                                    });
                                                state.tool_use_started.insert(event_index);
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStart {
                                                        index: event_index,
                                                        content_block: ContentBlockStart::ToolUse {
                                                            id,
                                                            name: name.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }

                                        if let Some(arguments) = args_opt {
                                            if !arguments.is_empty() {
                                                if state.tool_use_started.contains(&event_index) {
                                                    state.pending_events.push_back(
                                                        StreamEvent::ContentBlockDelta {
                                                            index: event_index,
                                                            delta: ContentDelta::InputJsonDelta {
                                                                partial_json: arguments.to_string(),
                                                            },
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }

                                // finish_reason（流结束原因）
                                if let Some(reason) = choice["finish_reason"].as_str() {
                                    // 先发送 ContentBlockStop（文本）
                                    if state.content_block_started {
                                        state.content_block_started = false;
                                        state
                                            .pending_events
                                            .push_back(StreamEvent::ContentBlockStop { index: 0 });
                                    }
                                    // 如果是工具调用结束，也关闭所有已开启的工具块
                                    if reason == "tool_calls" && !state.tool_use_started.is_empty()
                                    {
                                        let indices: Vec<usize> =
                                            state.tool_use_started.iter().copied().collect();
                                        for idx in indices {
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStop { index: idx },
                                            );
                                        }
                                        state.tool_use_started.clear();
                                    }

                                    // 然后发送 MessageDelta 带着 stop_reason
                                    let stop_reason = match reason {
                                        "stop" => Some(StopReason::EndTurn),
                                        "length" => Some(StopReason::MaxTokens),
                                        "tool_calls" => Some(StopReason::ToolUse),
                                        "content_filter" => Some(StopReason::EndTurn),
                                        _ => None,
                                    };

                                    state.pending_events.push_back(StreamEvent::MessageDelta {
                                        delta: MessageDeltaData {
                                            stop_reason,
                                            stop_sequence: None,
                                        },
                                        usage: Usage {
                                            input_tokens: 0,
                                            output_tokens: 0,
                                            cache_creation_input_tokens: None,
                                            cache_read_input_tokens: None,
                                        },
                                    });

                                    if let Some(evt) = state.pending_events.pop_front() {
                                        return Some((Ok(evt), (stream, state)));
                                    } else {
                                        continue;
                                    }
                                }

                                // 跳过其他 delta（如 role: "assistant"）
                                continue;
                            }
                            Some(Err(e)) => {
                                tracing::error!("OpenAI SSE stream error: {:?}", e);
                                return Some((
                                    Err(LlmProviderError::OpenAi(OpenAiError::Stream {
                                        message: format!("Network error: {}", e),
                                    })),
                                    (stream, state),
                                ));
                            }
                            None => return None, // 流结束
                        }
                    }
                }
            },
        );

        Ok(Box::pin(event_stream))
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

        let mut req_builder = self.client().post(&url).json(&body);
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
