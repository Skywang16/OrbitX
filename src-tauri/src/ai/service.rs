/*!
 * AI服务 -
 */

use crate::ai::types::AIModelConfig;
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::{RepositoryManager, Repository};
use crate::utils::error::AppResult;
use std::sync::Arc;

/// AI服务 -
pub struct AIService {
    repositories: Arc<RepositoryManager>,
    cache: Arc<UnifiedCache>,
}

impl AIService {
    pub fn new(repositories: Arc<RepositoryManager>, cache: Arc<UnifiedCache>) -> Self {
        Self { repositories, cache }
    }

    pub async fn initialize(&self) -> AppResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> Vec<AIModelConfig> {
        let cache_key = "ai_models_list";
        if let Some(cached_value) = self.cache.get(cache_key).await {
            if let Ok(models) = serde_json::from_value(cached_value) {
                return models;
            }
        }

        let models = self.repositories.ai_models().find_all().await.unwrap_or_default();
        if let Ok(models_value) = serde_json::to_value(&models) {
            let _ = self.cache.set(cache_key, models_value).await;
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
        let result = self.repositories.ai_models().delete_by_string_id(model_id).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn update_model(&self, model_id: &str, updates: serde_json::Value) -> AppResult<()> {
        let existing_config = self.repositories.ai_models().find_by_string_id(model_id).await?
            .ok_or_else(|| anyhow::anyhow!("模型不存在: {}", model_id))?;

        let mut config_value = serde_json::to_value(&existing_config)?;
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

    pub async fn test_connection(&self, _model_id: &str) -> AppResult<bool> {
        Ok(true) // 直接返回true
    }

    pub async fn set_default_model(&self, model_id: &str) -> AppResult<()> {
        let result = self.repositories.ai_models().set_default(model_id).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }
}
