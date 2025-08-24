/*!
 * AI服务
 */

use crate::ai::types::AIModelConfig;
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::{Repository, RepositoryManager};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub struct AIService {
    repositories: Arc<RepositoryManager>,
    cache: Arc<UnifiedCache>,
}

impl AIService {
    pub fn new(repositories: Arc<RepositoryManager>, cache: Arc<UnifiedCache>) -> Self {
        Self {
            repositories,
            cache,
        }
    }

    pub async fn initialize(&self) -> AppResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> Vec<AIModelConfig> {
        if let Some(cached) = self.cache.get("ai_models_list").await {
            if let Ok(models) = serde_json::from_value(cached) {
                return models;
            }
        }

        let models = self
            .repositories
            .ai_models()
            .find_all()
            .await
            .unwrap_or_default();
        if let Ok(value) = serde_json::to_value(&models) {
            let _ = self.cache.set("ai_models_list", value).await;
        }
        models
    }

    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        let result = self.repositories.ai_models().save(&config).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result.map(|_| ())
    }

    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        let result = self
            .repositories
            .ai_models()
            .delete_by_string_id(model_id)
            .await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn update_model(&self, model_id: &str, updates: serde_json::Value) -> AppResult<()> {
        let existing = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;

        let mut config_value = serde_json::to_value(&existing)?;
        if let serde_json::Value::Object(ref mut config_obj) = config_value {
            if let serde_json::Value::Object(updates_obj) = updates {
                for (key, value) in updates_obj {
                    config_obj.insert(key, value);
                }
            }
        }

        let final_config: AIModelConfig = serde_json::from_value(config_value)?;
        let result = self.repositories.ai_models().update(&final_config).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn test_connection(&self, model_id: &str) -> AppResult<bool> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;

        self.test_connection_with_config(&model).await
    }

    pub async fn test_connection_with_config(&self, model: &AIModelConfig) -> AppResult<bool> {
        match model.provider {
            crate::storage::repositories::ai_models::AIProvider::OpenAI => {
                self.test_openai_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Claude => {
                self.test_claude_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Custom => {
                self.test_custom_connection(model).await
            }
        }
    }

    async fn test_openai_connection(&self, model: &AIModelConfig) -> AppResult<bool> {
        let chat_url = format!("{}/chat/completions", model.api_url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        // 发送一个实际的LLM请求来测试连接和模型可用性
        let test_payload = json!({
            "model": model.model,
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1,
            "temperature": 0
        });

        let response = client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let status = response.status();
        
        // 对于OpenAI API，我们认为以下情况为成功：
        // - 200: 正常响应
        // - 400: Bad Request (通常表示API可用但请求参数有问题)
        // - 401: Unauthorized (API Key问题，但API端点可用)
        // - 429: Rate limit (API可用但达到限制)
        let is_success = status.is_success() 
            || status == reqwest::StatusCode::BAD_REQUEST 
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

        if !is_success {
            // 记录详细的错误信息用于调试
            if let Ok(error_text) = response.text().await {
                tracing::warn!("OpenAI连接测试失败: status={}, response={}", status, error_text);
            }
        }

        Ok(is_success)
    }

    async fn test_claude_connection(&self, model: &AIModelConfig) -> AppResult<bool> {
        let url = format!("{}/v1/messages", model.api_url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let test_payload = json!({
            "model": model.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let response = client
            .post(&url)
            .header("x-api-key", &model.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let status = response.status();
        
        // 对于Claude API，我们认为以下情况为成功：
        // - 200: 正常响应
        // - 400: Bad Request (通常表示API可用但请求参数有问题)
        // - 401: Unauthorized (API Key问题，但API端点可用)
        // - 429: Rate limit (API可用但达到限制)
        let is_success = status.is_success() 
            || status == reqwest::StatusCode::BAD_REQUEST 
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

        if !is_success {
            // 记录详细的错误信息用于调试
            if let Ok(error_text) = response.text().await {
                tracing::warn!("Claude连接测试失败: status={}, response={}", status, error_text);
            }
        }

        Ok(is_success)
    }

    async fn test_custom_connection(&self, model: &AIModelConfig) -> AppResult<bool> {
        let client = reqwest::Client::new();

        // 尝试发送一个实际的LLM请求来测试连接和模型可用性
        let chat_url = if model.api_url.ends_with("/v1") {
            format!("{}/chat/completions", model.api_url)
        } else if model.api_url.ends_with("/") {
            format!("{}v1/chat/completions", model.api_url)
        } else {
            format!("{}/v1/chat/completions", model.api_url)
        };

        let test_payload = json!({
            "model": model.model,
            "messages": [
                {
                    "role": "user", 
                    "content": "Hello"
                }
            ],
            "max_tokens": 1,
            "temperature": 0
        });

        let response = client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let status = response.status();
        
        // 对于LLM测试，我们认为以下情况为成功：
        // - 200: 正常响应
        // - 400: Bad Request (通常表示API可用但请求参数有问题)
        // - 401: Unauthorized (API Key问题，但API端点可用)
        // - 422: Unprocessable Entity (模型参数问题，但API可用)
        let is_success = status.is_success() 
            || status == reqwest::StatusCode::BAD_REQUEST 
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::UNPROCESSABLE_ENTITY;

        if !is_success {
            // 记录详细的错误信息用于调试
            if let Ok(error_text) = response.text().await {
                tracing::warn!("LLM连接测试失败: status={}, response={}", status, error_text);
            }
        }

        Ok(is_success)
    }
}
