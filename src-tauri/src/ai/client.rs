/*!
 * 简化的AI客户端
 *
 * 合并了原有的adapters和client功能到单一的AIClient
 * 保持现有API兼容性，仅重构内部实现
 */

use crate::ai::{
    AIModelConfig, AIProvider, AIRequest, AIResponse, AIResponseMetadata, AIResponseType,
    AIStreamResponse, AdapterCapabilities, BatchRequest, BatchResponse, HealthCheckRequest,
    HealthCheckResponse, ModelInfo, StreamChunk,
};
use crate::utils::error::{AppError, AppResult};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio_stream::wrappers::ReceiverStream;

// 使用async-openai库
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client as OpenAIClient,
};

/// 简化的AI客户端 - 统一所有AI提供商的接口
pub struct AIClient {
    config: AIModelConfig,
    openai_client: Option<OpenAIClient<OpenAIConfig>>,
    http_client: reqwest::Client,
}

impl AIClient {
    /// 创建新的AI客户端
    pub fn new(config: AIModelConfig) -> AppResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout()))
            .build()
            .with_context(|| format!("Failed to create HTTP client for model: {}", config.id))?;

        let openai_client = match config.provider {
            AIProvider::OpenAI | AIProvider::Local | AIProvider::Claude => {
                Some(Self::create_openai_client(&config)?)
            }
            AIProvider::Custom => None,
        };

        Ok(Self {
            config,
            openai_client,
            http_client,
        })
    }

    /// 创建OpenAI客户端配置
    fn create_openai_client(config: &AIModelConfig) -> AppResult<OpenAIClient<OpenAIConfig>> {
        use tracing::{debug, error, warn};

        debug!(
            "开始创建OpenAI客户端: model_id={}, provider={:?}",
            config.id, config.provider
        );
        debug!(
            "配置详情: api_url='{}', api_key_length={}, model='{}'",
            config.api_url,
            config.api_key.len(),
            config.model
        );

        // 验证必需的配置项
        if config.api_key.is_empty() {
            error!("AI配置错误 ({}): API密钥为空", config.id);
            return Err(anyhow!(
                "AI配置错误 ({}): API密钥不能为空，请在设置中配置有效的API密钥",
                config.id
            ));
        }

        if config.api_url.is_empty() {
            error!("AI配置错误 ({}): API地址为空", config.id);
            return Err(anyhow!(
                "AI配置错误 ({}): API地址不能为空，请在设置中配置有效的API地址",
                config.id
            ));
        }

        // 验证API密钥格式（基本检查）
        if config.provider == crate::ai::AIProvider::OpenAI && !config.api_key.starts_with("sk-") {
            warn!(
                "AI配置警告 ({}): OpenAI API密钥格式可能不正确，应以'sk-'开头",
                config.id
            );
        }

        let mut openai_config = OpenAIConfig::new().with_api_key(&config.api_key);

        // 如果是自定义API URL，设置base_url
        if !config.api_url.contains("api.openai.com") {
            debug!("使用自定义API地址: {}", config.api_url);
            let base_url = if config.api_url.contains("/v1") {
                let base = config
                    .api_url
                    .split("/v1")
                    .next()
                    .unwrap_or(&config.api_url);
                debug!("从完整URL提取base_url: {} -> {}", config.api_url, base);
                base
            } else {
                debug!("直接使用API地址作为base_url: {}", config.api_url);
                &config.api_url
            };

            if base_url.is_empty() {
                error!("AI配置错误 ({}): 处理后的base_url为空", config.id);
                return Err(anyhow!(
                    "AI配置错误 ({}): API地址处理失败，请检查API地址格式",
                    config.id
                ));
            }

            openai_config = openai_config.with_api_base(base_url);
        } else {
            debug!("使用默认OpenAI API地址");
        }

        debug!("OpenAI客户端配置创建完成，开始初始化客户端");

        let client = OpenAIClient::with_config(openai_config);
        debug!("OpenAI客户端创建成功: {}", config.id);
        Ok(client)
    }

    /// 构建ChatCompletion请求
    fn build_chat_request(
        &self,
        request: &AIRequest,
    ) -> Result<async_openai::types::CreateChatCompletionRequest, OpenAIError> {
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(request.content.clone())
            .build()?;

        let mut binding = CreateChatCompletionRequestArgs::default();
        let mut request_builder = binding
            .model(&self.config.model)
            .messages([user_message.into()]);

        // 设置可选参数
        if let Some(options) = &request.options {
            if let Some(max_tokens) = options.max_tokens {
                request_builder = request_builder.max_tokens(max_tokens);
            }
            if let Some(temperature) = options.temperature {
                request_builder = request_builder.temperature(temperature);
            }
            if let Some(stream) = options.stream {
                request_builder = request_builder.stream(stream);
            }
        }

        request_builder.build()
    }

    /// 将OpenAI错误转换为anyhow错误
    fn convert_openai_error(&self, error: OpenAIError) -> AppError {
        match error {
            OpenAIError::ApiError(api_error) => match api_error.r#type.as_deref() {
                Some("invalid_api_key") => {
                    anyhow!("AI认证失败 ({}): Invalid API key", self.config.id)
                }
                Some("insufficient_quota") | Some("rate_limit_exceeded") => {
                    anyhow!("AI请求频率限制 ({}): Rate limit exceeded", self.config.id)
                }
                _ => anyhow!(
                    "AI模型错误 ({}): API Error: {}",
                    self.config.id,
                    api_error.message
                ),
            },
            OpenAIError::Reqwest(req_error) => {
                anyhow!("AI网络连接错误 ({}): {}", self.config.id, req_error)
            }
            OpenAIError::JSONDeserialize(json_error) => {
                anyhow!(
                    "AI模型错误 ({}): JSON parsing error: {}",
                    self.config.id,
                    json_error
                )
            }
            _ => anyhow!("AI未知错误: {}", error),
        }
    }

    /// 处理OpenAI兼容的请求
    async fn handle_openai_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        let client = self.openai_client.as_ref().ok_or_else(|| {
            anyhow!(
                "AI配置错误 ({}): OpenAI client not initialized",
                self.config.id
            )
        })?;

        let chat_request = self
            .build_chat_request(request)
            .map_err(|e| self.convert_openai_error(e))?;

        let response = client
            .chat()
            .create(chat_request)
            .await
            .map_err(|e| self.convert_openai_error(e))?;

        // 解析响应
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("AI模型错误 ({}): No choices in response", self.config.id))?;

        let content =
            choice.message.content.as_ref().ok_or_else(|| {
                anyhow!("AI模型错误 ({}): No content in response", self.config.id)
            })?;

        // 智能推断响应类型
        let response_type = if content.contains("```") {
            AIResponseType::Code
        } else if content.starts_with("$") || content.starts_with("sudo") {
            AIResponseType::Command
        } else {
            AIResponseType::Text
        };

        Ok(AIResponse {
            content: content.clone(),
            response_type,
            suggestions: None,
            metadata: Some(AIResponseMetadata {
                model: Some(response.model.clone()),
                tokens_used: response.usage.map(|usage| usage.total_tokens),
                response_time: None,
            }),
        })
    }

    /// 处理OpenAI兼容的流式请求
    async fn handle_openai_stream_request(
        &self,
        request: &AIRequest,
    ) -> AppResult<AIStreamResponse> {
        let client = self.openai_client.as_ref().ok_or_else(|| {
            anyhow!(
                "AI配置错误 ({}): OpenAI client not initialized",
                self.config.id
            )
        })?;

        let mut chat_request = self
            .build_chat_request(request)
            .map_err(|e| self.convert_openai_error(e))?;
        chat_request.stream = Some(true);

        let mut stream = client
            .chat()
            .create_stream(chat_request)
            .await
            .map_err(|e| self.convert_openai_error(e))?;

        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            // 发送流式响应开始信号
            let start_chunk = StreamChunk {
                content: String::new(),
                is_complete: false,
                metadata: Some({
                    let mut metadata = HashMap::new();
                    metadata.insert("stream_started".to_string(), serde_json::Value::Bool(true));
                    metadata
                }),
            };
            if tx.send(Ok(start_chunk)).await.is_err() {
                return;
            }

            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            let content = choice.delta.content.as_deref().unwrap_or("");
                            let is_complete = choice.finish_reason.is_some();

                            let chunk = StreamChunk {
                                content: content.to_string(),
                                is_complete,
                                metadata: choice.finish_reason.as_ref().map(|reason| {
                                    let mut metadata = HashMap::new();
                                    metadata.insert(
                                        "finish_reason".to_string(),
                                        serde_json::Value::String(format!("{:?}", reason)),
                                    );
                                    metadata
                                }),
                            };

                            if tx.send(Ok(chunk)).await.is_err() {
                                break;
                            }

                            if is_complete {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let error = anyhow!("AI模型错误: {}", e);
                        let _ = tx.send(Err(error)).await;
                        break;
                    }
                }
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    /// 处理自定义API请求
    async fn handle_custom_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        // 构建请求体
        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": request.content
                }
            ]
        });

        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(max_tokens) = options.max_tokens {
                request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
            }
            if let Some(temperature) = options.temperature {
                request_body["temperature"] = serde_json::Value::Number(
                    serde_json::Number::from_f64(temperature as f64)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                );
            }
            if let Some(stream) = options.stream {
                request_body["stream"] = serde_json::Value::Bool(stream);
            }
        }

        // 构建请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .with_context(|| {
                    format!("AI配置错误 ({}): Invalid API key format", self.config.id)
                })?,
        );

        // 发送请求
        let response = self
            .http_client
            .post(&self.config.api_url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .with_context(|| format!("AI网络连接错误 ({})", self.config.id))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "AI模型错误 ({}): HTTP {}: {}",
                self.config.id,
                status,
                error_text
            ));
        }

        // 解析响应
        let response_json: serde_json::Value = response.json().await.with_context(|| {
            format!("AI模型错误 ({}): Failed to parse response", self.config.id)
        })?;

        // 提取内容
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| anyhow!("AI模型错误 ({}): Invalid response format", self.config.id))?;

        // 智能推断响应类型
        let response_type = if content.contains("```") {
            AIResponseType::Code
        } else if content.starts_with("$") || content.starts_with("sudo") {
            AIResponseType::Command
        } else {
            AIResponseType::Text
        };

        Ok(AIResponse {
            content: content.to_string(),
            response_type,
            suggestions: None,
            metadata: Some(AIResponseMetadata {
                model: Some(self.config.model.clone()),
                tokens_used: response_json
                    .get("usage")
                    .and_then(|usage| usage.get("total_tokens"))
                    .and_then(|tokens| tokens.as_u64())
                    .map(|tokens| tokens as u32),
                response_time: None,
            }),
        })
    }

    /// 获取模型配置
    pub fn config(&self) -> &AIModelConfig {
        &self.config
    }

    /// 更新模型配置
    pub fn update_config(&mut self, config: AIModelConfig) -> AppResult<()> {
        // 如果提供商改变，需要重新创建客户端
        if config.provider != self.config.provider
            || config.api_url != self.config.api_url
            || config.api_key != self.config.api_key
        {
            self.openai_client = match config.provider {
                AIProvider::OpenAI | AIProvider::Local | AIProvider::Claude => {
                    Some(Self::create_openai_client(&config)?)
                }
                AIProvider::Custom => None,
            };

            self.http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout()))
                .build()
                .with_context(|| {
                    format!("AI配置错误 ({}): Failed to create HTTP client", config.id)
                })?;
        }

        self.config = config;
        Ok(())
    }
}

/// AI适配器trait - 保持向后兼容性
#[async_trait]
pub trait AIAdapter: Send + Sync {
    /// 发送请求到AI模型
    async fn send_request(&self, request: &AIRequest) -> AppResult<AIResponse>;

    /// 发送流式请求
    async fn send_stream_request(&self, request: &AIRequest) -> AppResult<AIStreamResponse>;

    /// 测试连接
    async fn test_connection(&self) -> AppResult<bool>;

    /// 获取适配器名称
    fn name(&self) -> &str;

    /// 获取支持的功能
    fn supported_features(&self) -> Vec<String>;

    /// 获取适配器能力信息
    fn get_capabilities(&self) -> AdapterCapabilities;

    /// 健康检查
    async fn health_check(&self, request: &HealthCheckRequest) -> AppResult<HealthCheckResponse>;

    /// 获取模型信息
    async fn get_model_info(&self) -> AppResult<ModelInfo>;

    /// 发送批量请求
    async fn send_batch_request(&self, batch: &BatchRequest) -> AppResult<BatchResponse>;

    /// 取消请求
    async fn cancel_request(&self, request_id: &str) -> AppResult<()>;
}

/// 为AIClient实现AIAdapter trait以保持向后兼容性
#[async_trait]
impl AIAdapter for AIClient {
    async fn send_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        match self.config.provider {
            AIProvider::OpenAI | AIProvider::Local | AIProvider::Claude => {
                self.handle_openai_request(request).await
            }
            AIProvider::Custom => self.handle_custom_request(request).await,
        }
    }

    async fn send_stream_request(&self, request: &AIRequest) -> AppResult<AIStreamResponse> {
        match self.config.provider {
            AIProvider::OpenAI | AIProvider::Local | AIProvider::Claude => {
                self.handle_openai_stream_request(request).await
            }
            AIProvider::Custom => {
                // 自定义提供商暂不支持流式
                Err(anyhow!(
                    "AI未知错误: Streaming not supported for custom providers"
                ))
            }
        }
    }

    async fn test_connection(&self) -> AppResult<bool> {
        let test_request = AIRequest::chat("Hello".to_string());
        match self.send_request(&test_request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("认证失败") || error_msg.contains("网络连接错误") {
                    Ok(false)
                } else {
                    Ok(true) // 其他错误认为连接正常
                }
            }
        }
    }

    fn name(&self) -> &str {
        match self.config.provider {
            AIProvider::OpenAI => "OpenAI",
            AIProvider::Claude => "Claude",
            AIProvider::Local => "Local",
            AIProvider::Custom => "Custom",
        }
    }

    fn supported_features(&self) -> Vec<String> {
        vec![
            "completion".to_string(),
            "chat".to_string(),
            "explanation".to_string(),
            "error-analysis".to_string(),
        ]
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            supports_streaming: matches!(
                self.config.provider,
                AIProvider::OpenAI | AIProvider::Local | AIProvider::Claude
            ),
            supports_batch: false,
            supports_function_calling: false,
            supports_vision: false,
            max_tokens: Some(self.config.max_tokens()),
            max_batch_size: None,
            supported_models: vec![self.config.model.clone()],
        }
    }

    async fn health_check(&self, _request: &HealthCheckRequest) -> AppResult<HealthCheckResponse> {
        let start_time = Instant::now();
        let is_healthy = self.test_connection().await?;
        let latency = start_time.elapsed().as_millis() as u64;

        Ok(HealthCheckResponse {
            status: if is_healthy {
                crate::ai::HealthStatus::Healthy
            } else {
                crate::ai::HealthStatus::Unhealthy
            },
            latency: Some(latency),
            model_info: Some(ModelInfo {
                name: self.config.model.clone(),
                version: None,
                context_length: Some(self.config.max_tokens()),
                capabilities: Some(self.get_capabilities()),
            }),
            error: None,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn get_model_info(&self) -> AppResult<ModelInfo> {
        Ok(ModelInfo {
            name: self.config.model.clone(),
            version: None,
            context_length: Some(self.config.max_tokens()),
            capabilities: Some(self.get_capabilities()),
        })
    }

    async fn send_batch_request(&self, _batch: &BatchRequest) -> AppResult<BatchResponse> {
        Err(anyhow!("AI未知错误: Batch requests not supported"))
    }

    async fn cancel_request(&self, _request_id: &str) -> AppResult<()> {
        Err(anyhow!("AI未知错误: Request cancellation not supported"))
    }
}

/// 统一的AI适配器 - 为了向后兼容性保留
pub type UnifiedAIAdapter = AIClient;

/// 自定义适配器 - 为了向后兼容性保留
pub type CustomAdapter = AIClient;
