/*!
 * AI服务
 */

use crate::ai::types::AIModelConfig;
use crate::storage::repositories::{Repository, RepositoryManager};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub struct AIService {
    repositories: Arc<RepositoryManager>,
}

impl AIService {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    pub async fn initialize(&self) -> AppResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> Vec<AIModelConfig> {
        self.repositories
            .ai_models()
            .find_all()
            .await
            .unwrap_or_default()
    }

    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        self.repositories
            .ai_models()
            .save(&config)
            .await
            .map(|_| ())
    }

    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        self.repositories
            .ai_models()
            .delete_by_string_id(model_id)
            .await
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
        self.repositories.ai_models().update(&final_config).await
    }

    pub async fn test_connection(&self, model_id: &str) -> AppResult<String> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;

        self.test_connection_with_config(&model).await
    }

    pub async fn test_connection_with_config(&self, model: &AIModelConfig) -> AppResult<String> {
        match model.provider {
            crate::storage::repositories::ai_models::AIProvider::OpenAI => {
                self.test_openai_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Claude => {
                self.test_claude_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Gemini => {
                self.test_gemini_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Qwen => {
                self.test_qwen_connection(model).await
            }
            crate::storage::repositories::ai_models::AIProvider::Custom => {
                self.test_custom_connection(model).await
            }
        }
    }

    async fn test_openai_connection(&self, model: &AIModelConfig) -> AppResult<String> {
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

        let response = match client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await {
                Ok(response) => response,
                Err(e) => return Ok(format!("连接失败: {}", e)),
            };

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

        if is_success {
            Ok("连接成功".to_string())
        } else {
            // 抛出详细的错误信息
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取响应内容".to_string());
            let error_msg = format!("OpenAI API 错误: {} - {}", status, error_text);
            tracing::warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    async fn test_claude_connection(&self, model: &AIModelConfig) -> AppResult<String> {
        let url = format!("{}/v1/messages", model.api_url.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let test_payload = json!({
            "model": model.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let response = match client
            .post(&url)
            .header("x-api-key", &model.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await {
                Ok(response) => response,
                Err(e) => return Ok(format!("连接失败: {}", e)),
            };

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

        if is_success {
            Ok("连接成功".to_string())
        } else {
            // 抛出详细的错误信息
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取响应内容".to_string());
            let error_msg = format!("Claude API 错误: {} - {}", status, error_text);
            tracing::warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    async fn test_custom_connection(&self, model: &AIModelConfig) -> AppResult<String> {
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

        let response = match client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await {
                Ok(response) => response,
                Err(e) => return Ok(format!("连接失败: {}", e)),
            };

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

        if is_success {
            Ok("连接成功".to_string())
        } else {
            // 抛出详细的错误信息
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取响应内容".to_string());
            let error_msg = format!("自定义 API 错误: {} - {}", status, error_text);
            tracing::warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    async fn test_gemini_connection(&self, model: &AIModelConfig) -> AppResult<String> {
        use crate::llm::providers::base::LLMProvider;
        use crate::llm::providers::gemini::GeminiProvider;
        use crate::llm::types::{
            LLMMessage, LLMMessageContent, LLMProviderConfig, LLMProviderType, LLMRequest,
        };

        // 直接使用 GeminiProvider 进行测试，确保一致性
        let config = LLMProviderConfig {
            provider_type: LLMProviderType::Gemini,
            api_url: if model.api_url.trim().is_empty() {
                None
            } else {
                Some(model.api_url.clone())
            },
            api_key: model.api_key.clone(),
            model: model.model.clone(),
            options: None,
        };

        let provider = GeminiProvider::new(config);

        tracing::debug!(
            "开始Gemini连接测试，模型: {}, API URL: '{}'",
            model.model,
            model.api_url
        );

        let test_request = LLMRequest {
            model: model.model.clone(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("Hello".to_string()),
            }],
            temperature: Some(0.0),
            max_tokens: Some(1),
            stream: false,
            tools: None,
            tool_choice: None,
        };

        tracing::debug!("发送Gemini测试请求...");
        match provider.call(test_request).await {
            Ok(response) => {
                tracing::debug!("Gemini连接测试成功，响应: {:?}", response);
                Ok("连接成功".to_string())
            }
            Err(e) => {
                let error_msg = format!("Gemini API 错误: {}", e);
                tracing::error!("Gemini连接测试失败，错误: {}", e);
                tracing::error!("错误详情: {:?}", e);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn test_qwen_connection(&self, model: &AIModelConfig) -> AppResult<String> {
        let chat_url = format!(
            "{}/v1/chat/completions",
            model.api_url.trim_end_matches('/')
        );
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

        let response = match client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", model.api_key))
            .header("Content-Type", "application/json")
            .json(&test_payload)
            .timeout(Duration::from_secs(15))
            .send()
            .await {
                Ok(response) => response,
                Err(e) => return Ok(format!("连接失败: {}", e)),
            };

        let status = response.status();

        // 对于Qwen API，我们认为以下情况为成功：
        // - 200: 正常响应
        // - 400: Bad Request (通常表示API可用但请求参数有问题)
        // - 401: Unauthorized (API Key问题，但API端点可用)
        // - 429: Rate limit (API可用但达到限制)
        let is_success = status.is_success()
            || status == reqwest::StatusCode::BAD_REQUEST
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

        if is_success {
            Ok("连接成功".to_string())
        } else {
            // 抛出详细的错误信息
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取响应内容".to_string());
            let error_msg = format!("Qwen API 错误: {} - {}", status, error_text);
            tracing::warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }
}
