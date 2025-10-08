use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::llm::{
    providers::ProviderFactory,
    types::{
        EmbeddingRequest, EmbeddingResponse, LLMProviderConfig, LLMProviderType, LLMRequest,
        LLMResponse, LLMStreamChunk,
    },
};
use crate::storage::repositories::RepositoryManager;
use anyhow::{Context, Result};

pub struct LLMService {
    repositories: Arc<RepositoryManager>,
}

impl LLMService {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    async fn get_provider_config(&self, model_id: &str) -> Result<LLMProviderConfig> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await
            .with_context(|| format!("查找模型失败: {}", model_id))?
            .ok_or_else(|| anyhow::anyhow!("模型未找到: {}", model_id))?;

        let provider_str = model.provider.to_string().to_lowercase();
        let provider_type = match provider_str.as_str() {
            "openai" => LLMProviderType::OpenAI,
            "claude" | "anthropic" => LLMProviderType::Anthropic,
            "gemini" => LLMProviderType::Gemini,
            "qwen" => LLMProviderType::Qwen,
            "custom" => LLMProviderType::Custom,
            _ => anyhow::bail!("不支持的提供商类型: {}", model.provider),
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
                    .context("解析模型选项失败")?,
                ),
                None => None,
            },
        })
    }

    /// 非流式调用
    pub async fn call(&self, request: LLMRequest) -> Result<LLMResponse> {
        self.validate_request(&request)?;
        let original_model_id = request.model.clone();
        let config = self.get_provider_config(&request.model).await?;
        let provider = ProviderFactory::create_provider(config.clone())?;

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

    /// 流式调用（携带外部取消令牌）
    pub async fn call_stream(
        &self,
        request: LLMRequest,
        token: CancellationToken,
    ) -> Result<impl tokio_stream::Stream<Item = Result<LLMStreamChunk>>> {
        self.validate_request(&request)?;
        let original_model_id = request.model.clone();
        let config = self.get_provider_config(&request.model).await?;
        let provider = ProviderFactory::create_provider(config.clone())?;

        let mut actual_request = request.clone();
        actual_request.model = config.model.clone();

        tracing::debug!(
            "Making streaming LLM call with model: {} (config: {}), with external cancel token",
            actual_request.model,
            original_model_id
        );

        let stream = provider.call_stream(actual_request).await?;

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
    pub async fn create_embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let original_model_id = request.model.clone();
        let config = self.get_provider_config(&request.model).await?;
        let provider = ProviderFactory::create_provider(config.clone())?;

        let mut actual_request = request.clone();
        actual_request.model = config.model.clone();

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

        result
    }

    /// 获取可用的模型列表
    pub async fn get_available_models(&self) -> Result<Vec<String>> {
        let models = self
            .repositories
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .context("获取模型列表失败")?;

        Ok(models.into_iter().map(|m| m.id).collect())
    }

    /// 测试模型连接
    pub async fn test_model_connection(&self, model_id: &str) -> Result<bool> {
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
    fn validate_request(&self, request: &LLMRequest) -> Result<()> {
        if request.model.is_empty() {
            anyhow::bail!("模型名称不能为空");
        }

        if request.messages.is_empty() {
            anyhow::bail!("消息列表不能为空");
        }

        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 2.0 {
                anyhow::bail!("温度参数必须在 0.0 到 2.0 之间");
            }
        }

        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                anyhow::bail!("最大令牌数必须大于 0");
            }
        }

        Ok(())
    }
}
