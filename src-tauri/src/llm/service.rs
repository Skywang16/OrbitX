use std::sync::Arc;

use crate::llm::{
    providers::ProviderFactory,
    types::{
        LLMError, LLMProviderConfig, LLMProviderType, LLMRequest, LLMResponse, LLMResult,
        LLMStreamChunk,
    },
};
use crate::storage::repositories::RepositoryManager;

/// LLM 服务
pub struct LLMService {
    repositories: Arc<RepositoryManager>,
}

impl LLMService {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    /// 根据模型ID获取提供商配置
    async fn get_provider_config(&self, model_id: &str) -> LLMResult<LLMProviderConfig> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await
            .map_err(|e| LLMError::Config(format!("Failed to find model: {}", e)))?
            .ok_or_else(|| LLMError::ModelNotFound(model_id.to_string()))?;

        let provider_str = model.provider.to_string().to_lowercase();
        let provider_type = match provider_str.as_str() {
            "openai" => LLMProviderType::OpenAI,
            "claude" | "anthropic" => LLMProviderType::Anthropic,
            "gemini" => LLMProviderType::Gemini,
            "qwen" => LLMProviderType::Qwen,
            "custom" => LLMProviderType::Custom,
            _ => return Err(LLMError::UnsupportedProvider(model.provider.to_string())),
        };

        Ok(LLMProviderConfig {
            provider_type,
            api_key: model.api_key,
            api_url: if model.api_url.is_empty() {
                None
            } else {
                Some(model.api_url)
            },
            model: model.model,
            options: match model.options {
                Some(v) => Some(
                    serde_json::from_value::<std::collections::HashMap<String, serde_json::Value>>(
                        v,
                    )
                    .map_err(|e| LLMError::Config(format!("Invalid options: {}", e)))?,
                ),
                None => None,
            },
        })
    }

    /// 非流式调用
    pub async fn call(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        self.validate_request(&request)?;
        let original_model_id = request.model.clone();
        let config = self.get_provider_config(&request.model).await?;
        let provider = ProviderFactory::create_provider(config.clone())?;

        // 创建新的请求，使用真实的模型名称而不是数据库ID
        let mut actual_request = request.clone();
        actual_request.model = config.model.clone();

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

        result
    }

    /// 流式调用
    pub async fn call_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<impl tokio_stream::Stream<Item = LLMResult<LLMStreamChunk>>> {
        self.validate_request(&request)?;
        let original_model_id = request.model.clone();
        let config = self.get_provider_config(&request.model).await?;
        let provider = ProviderFactory::create_provider(config.clone())?;

        // 创建新的请求，使用真实的模型名称而不是数据库ID
        let mut actual_request = request.clone();
        actual_request.model = config.model.clone();

        tracing::debug!(
            "Making streaming LLM call with model: {} (config: {})",
            actual_request.model,
            original_model_id
        );
        let stream = provider.call_stream(actual_request).await?;
        Ok(stream)
    }

    /// 获取可用的模型列表
    pub async fn get_available_models(&self) -> LLMResult<Vec<String>> {
        let models = self
            .repositories
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .map_err(|e| LLMError::Config(format!("Failed to get models: {}", e)))?;

        Ok(models.into_iter().map(|m| m.id).collect())
    }

    /// 测试模型连接
    pub async fn test_model_connection(&self, model_id: &str) -> LLMResult<bool> {
        let test_request = LLMRequest {
            model: model_id.to_string(),
            messages: vec![super::types::LLMMessage {
                role: "user".to_string(),
                content: super::types::LLMMessageContent::Text("Hello".to_string()),
            }],
            temperature: Some(0.1),
            max_tokens: Some(10),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        match self.call(test_request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("Model connection test failed for {}: {}", model_id, e);
                Ok(false)
            }
        }
    }

    /// 验证请求参数
    fn validate_request(&self, request: &LLMRequest) -> LLMResult<()> {
        if request.model.is_empty() {
            return Err(LLMError::Config("Model cannot be empty".to_string()));
        }

        if request.messages.is_empty() {
            return Err(LLMError::Config("Messages cannot be empty".to_string()));
        }

        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(LLMError::Config(
                    "Temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }

        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(LLMError::Config(
                    "Max tokens must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}
