use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::llm::{
    anthropic_types::{CreateMessageRequest, Message, MessageContent, MessageParam, StreamEvent},
    error::{LlmError, LlmProviderResult, LlmResult},
    provider_registry::ProviderRegistry,
    types::{EmbeddingRequest, EmbeddingResponse, LLMProviderConfig},
};
use crate::storage::DatabaseManager;
use crate::storage::repositories::AIModels;

pub struct LLMService {
    database: Arc<DatabaseManager>,
}

impl LLMService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    async fn get_provider_config(&self, model_id: &str) -> LlmResult<LLMProviderConfig> {
        let model = AIModels::new(&self.database)
            .find_by_id(model_id)
            .await?
            .ok_or_else(|| LlmError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;

        let provider_type = model.provider.as_str().to_string();

        if !ProviderRegistry::global().supports(&provider_type) {
            return Err(LlmError::UnsupportedProvider {
                provider: provider_type.clone(),
            });
        }

        let options = match model.options {
            Some(value) => Some(
                serde_json::from_value::<std::collections::HashMap<String, serde_json::Value>>(
                    value,
                )
                .map_err(|source| LlmError::OptionsParse { source })?,
            ),
            None => None,
        };

        // 从 options 中提取 supportsPromptCache
        let supports_prompt_cache = options
            .as_ref()
            .and_then(|opts| opts.get("supportsPromptCache"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(LLMProviderConfig {
            provider_type,
            api_key: model.api_key,
            api_url: if model.api_url.is_empty() {
                None
            } else {
                Some(model.api_url)
            },
            model: model.model,
            options,
            supports_prompt_cache,
        })
    }

    /// 非流式调用（Anthropic 原生类型）
    pub async fn call(&self, request: CreateMessageRequest) -> LlmResult<Message> {
        self.validate_request(&request)?;
        let mut config = self.get_provider_config(&request.model).await?;
        // ✅ Move model out before passing config to create (avoid clone)
        let provider_model = std::mem::take(&mut config.model);
        let provider = ProviderRegistry::global()
            .create(config)
            .map_err(LlmError::from)?;

        let mut actual_request = request;
        let original_model_id = std::mem::replace(&mut actual_request.model, provider_model);

        tracing::debug!(
            "Making LLM call with model: {} (config: {})",
            actual_request.model,
            original_model_id
        );
        let result = provider.call(actual_request).await;

        match &result {
            Ok(response) => {
                tracing::debug!(
                    "LLM call successful, response length: {}",
                    response.content.len()
                );
            }
            Err(e) => {
                tracing::error!("LLM call failed: {}", e);
            }
        }

        result.map_err(LlmError::from)
    }

    /// 流式调用（携带外部取消令牌，Anthropic 原生类型）
    pub async fn call_stream(
        &self,
        request: CreateMessageRequest,
        token: CancellationToken,
    ) -> LlmResult<impl tokio_stream::Stream<Item = LlmProviderResult<StreamEvent>>> {
        self.validate_request(&request)?;
        let mut config = self.get_provider_config(&request.model).await?;
        // ✅ Move model out before passing config to create (avoid clone)
        let provider_model = std::mem::take(&mut config.model);
        let provider = ProviderRegistry::global()
            .create(config)
            .map_err(LlmError::from)?;

        let mut actual_request = request;
        // ✅ Move provider model into request (zero-cost)
        let original_model_id = std::mem::replace(&mut actual_request.model, provider_model);

        tracing::debug!(
            "Making streaming LLM call with model: {} (config: {}), with external cancel token",
            actual_request.model,
            original_model_id
        );

        let stream = provider
            .call_stream(actual_request)
            .await
            .map_err(LlmError::from)?;

        let stream_with_cancel = tokio_stream::wrappers::ReceiverStream::new({
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let mut stream = Box::pin(stream);

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::debug!("Stream cancelled by token.");
                            break;
                        }
                        item = stream.next() => {
                            if let Some(item) = item {
                                if tx.send(item).await.is_err() {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            });
            rx
        });

        Ok(stream_with_cancel)
    }

    /// Embedding调用
    pub async fn create_embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> LlmResult<EmbeddingResponse> {
        let mut config = self.get_provider_config(&request.model).await?;
        let provider_model = std::mem::take(&mut config.model);
        let provider = ProviderRegistry::global()
            .create(config)
            .map_err(LlmError::from)?;

        let mut actual_request = request;
        let original_model_id = std::mem::replace(&mut actual_request.model, provider_model);

        tracing::debug!(
            "Making embedding call with model: {} (config: {})",
            actual_request.model,
            original_model_id
        );

        let result = provider.create_embeddings(actual_request).await;

        match &result {
            Ok(response) => {
                tracing::debug!(
                    "Embedding call successful, {} vectors generated",
                    response.data.len()
                );
            }
            Err(e) => {
                tracing::error!("Embedding call failed: {}", e);
            }
        }

        result.map_err(LlmError::from)
    }

    /// 获取可用的模型列表
    pub async fn get_available_models(&self) -> LlmResult<Vec<String>> {
        let ai_models = AIModels::new(&self.database);
        let models = ai_models.find_all().await?;

        Ok(models.into_iter().map(|m| m.id).collect())
    }

    /// 测试模型连接（构造最简 Anthropic CreateMessageRequest）
    pub async fn test_model_connection(&self, model_id: &str) -> LlmResult<bool> {
        let test_request = CreateMessageRequest {
            model: model_id.to_string(),
            messages: std::collections::VecDeque::from(vec![MessageParam {
                role: crate::llm::anthropic_types::MessageRole::User,
                content: MessageContent::Text("Hello".to_string()),
            }]),
            max_tokens: 10,
            system: None,
            tools: None,
            temperature: Some(0.1),
            stream: false,
            stop_sequences: None,
            top_p: None,
            top_k: None,
            metadata: None,
        };

        let result = self.call(test_request).await;
        match result {
            Ok(_) => Ok(true),
            Err(err) => {
                tracing::warn!("Model connection test failed for {}: {}", model_id, err);
                Ok(false)
            }
        }
    }

    /// 验证请求参数（Anthropic 原生类型）
    fn validate_request(&self, request: &CreateMessageRequest) -> LlmResult<()> {
        if request.model.is_empty() {
            return Err(LlmError::InvalidRequest {
                reason: "Model identifier cannot be empty".to_string(),
            });
        }

        if request.messages.is_empty() {
            return Err(LlmError::InvalidRequest {
                reason: "Message list cannot be empty".to_string(),
            });
        }

        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(LlmError::InvalidRequest {
                    reason: "Temperature must be between 0.0 and 2.0".to_string(),
                });
            }
        }

        if request.max_tokens == 0 {
            return Err(LlmError::InvalidRequest {
                reason: "max_tokens must be greater than zero".to_string(),
            });
        }

        Ok(())
    }
}
