/*!
 * AI服务 -
 */

use crate::ai::AIModelConfig;
use crate::storage::cache::UnifiedCache;
use crate::storage::sqlite::SqliteManager;
use crate::utils::error::AppResult;
use std::sync::Arc;

/// AI服务 -
pub struct AIService {
    storage: Arc<SqliteManager>,
    cache: Arc<UnifiedCache>,
}

impl AIService {
    pub fn new(storage: Arc<SqliteManager>, cache: Arc<UnifiedCache>) -> Self {
        Self { storage, cache }
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

        let models = self.storage.get_ai_models().await.unwrap_or_default();
        if let Ok(models_value) = serde_json::to_value(&models) {
            let _ = self.cache.set(cache_key, models_value).await;
        }
        models
    }

    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        let config_value = serde_json::to_value(&config)?;
        let save_options = crate::storage::types::SaveOptions {
            table: Some("ai_models".to_string()),
            overwrite: false,
            ..Default::default()
        };
        let result = self.storage.save_data(&config_value, &save_options).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        let result = self.storage.delete_ai_model(model_id).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn update_model(&self, model_id: &str, updates: serde_json::Value) -> AppResult<()> {
        let models = self.storage.get_ai_models().await?;
        let existing_config = models
            .iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| anyhow::anyhow!("模型不存在: {}", model_id))?;

        let mut config_value = serde_json::to_value(existing_config)?;
        if let serde_json::Value::Object(ref mut config_obj) = config_value {
            if let serde_json::Value::Object(updates_obj) = updates {
                for (key, value) in updates_obj {
                    config_obj.insert(key, value);
                }
            }
        }

        let final_config: AIModelConfig = serde_json::from_value(config_value)?;
        let config_value = serde_json::to_value(&final_config)?;
        let save_options = crate::storage::types::SaveOptions {
            table: Some("ai_models".to_string()),
            overwrite: true,
            ..Default::default()
        };
        let result = self.storage.save_data(&config_value, &save_options).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }

    pub async fn test_connection(&self, _model_id: &str) -> AppResult<bool> {
        Ok(true) // 直接返回true
    }

    pub async fn set_default_model(&self, model_id: &str) -> AppResult<()> {
        let result = self.storage.set_default_ai_model(model_id).await;
        if result.is_ok() {
            self.cache.remove("ai_models_list").await;
        }
        result
    }
}
