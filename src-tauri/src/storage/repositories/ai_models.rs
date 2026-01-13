/*!
 * AI 模型数据访问
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::RepositoryResult;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use tracing::error;

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AIProvider {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

impl AIProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::Anthropic => "anthropic",
            AIProvider::OpenAiCompatible => "openai_compatible",
        }
    }
}

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AIProvider {
    type Err = crate::storage::error::RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(AIProvider::Anthropic),
            "openai_compatible" => Ok(AIProvider::OpenAiCompatible),
            _ => Err(crate::storage::error::RepositoryError::Validation {
                reason: format!("Unknown AI provider: {s}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum ModelType {
    #[serde(rename = "chat")]
    #[default]
    Chat,
    #[serde(rename = "embedding")]
    Embedding,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Chat => write!(f, "chat"),
            ModelType::Embedding => write!(f, "embedding"),
        }
    }
}

impl std::str::FromStr for ModelType {
    type Err = crate::storage::error::RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(ModelType::Chat),
            "embedding" => Ok(ModelType::Embedding),
            _ => Err(crate::storage::error::RepositoryError::Validation {
                reason: format!("Unknown model type: {s}"),
            }),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    pub provider: AIProvider,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub model_type: ModelType,
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default)]
    pub use_custom_base_url: Option<bool>,
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
}

impl AIModelConfig {
    pub fn new(provider: AIProvider, api_url: String, api_key: String, model: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            api_url,
            api_key,
            model,
            model_type: ModelType::Chat,
            options: None,
            use_custom_base_url: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_model_type(
        provider: AIProvider,
        api_url: String,
        api_key: String,
        model: String,
        model_type: ModelType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            api_url,
            api_key,
            model,
            model_type,
            options: None,
            use_custom_base_url: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// AI 模型数据访问结构体
pub struct AIModels<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AIModels<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    /// 查询所有模型（包含解密的密钥）
    pub async fn find_all(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, api_url, api_key_encrypted, model_name, model_type,
                   config_json, use_custom_base_url, created_at, updated_at
            FROM ai_models
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut models = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let provider_str: String = row.try_get("provider")?;
            let provider = provider_str.parse()?;
            let model_type_str: String = row.try_get("model_type")?;
            let model_type = model_type_str.parse()?;

            let options = row
                .try_get::<Option<String>, _>("config_json")?
                .and_then(|s| serde_json::from_str(&s).ok());

            let use_custom_base_url = row
                .try_get::<Option<i64>, _>("use_custom_base_url")?
                .map(|v| v != 0);

            // 解密 API 密钥
            let api_key = if let Some(encrypted_base64) =
                row.try_get::<Option<String>, _>("api_key_encrypted")?
            {
                if !encrypted_base64.is_empty() {
                    match BASE64.decode(&encrypted_base64) {
                        Ok(encrypted_bytes) => self
                            .db
                            .decrypt_data(&encrypted_bytes)
                            .await
                            .unwrap_or_else(|e| {
                                error!("解密API密钥失败 ({}): {}", id, e);
                                String::new()
                            }),
                        Err(e) => {
                            error!("Base64解码失败 ({}): {}", id, e);
                            String::new()
                        }
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            models.push(AIModelConfig {
                id,
                provider,
                api_url: row.try_get("api_url")?,
                api_key,
                model: row.try_get("model_name")?,
                model_type,
                options,
                use_custom_base_url,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(models)
    }

    /// 保存模型（自动加密密钥）
    pub async fn save(&self, model: &AIModelConfig) -> RepositoryResult<()> {
        // 加密 API 密钥
        let encrypted_key = if !model.api_key.is_empty() {
            let encrypted_bytes = self.db.encrypt_data(&model.api_key).await?;
            Some(BASE64.encode(&encrypted_bytes))
        } else {
            // 保留现有密钥
            sqlx::query_scalar("SELECT api_key_encrypted FROM ai_models WHERE id = ?")
                .bind(&model.id)
                .fetch_optional(self.db.pool())
                .await?
        };

        let config_json = model
            .options
            .as_ref()
            .map(|opts| serde_json::to_string(opts).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO ai_models 
            (id, provider, api_url, api_key_encrypted, model_name, model_type, 
             config_json, use_custom_base_url, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&model.id)
        .bind(model.provider.to_string())
        .bind(&model.api_url)
        .bind(encrypted_key)
        .bind(&model.model)
        .bind(model.model_type.to_string())
        .bind(config_json)
        .bind(model.use_custom_base_url.map(|v| v as i64))
        .bind(model.created_at)
        .bind(model.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// 根据 ID 查找模型
    pub async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<AIModelConfig>> {
        let models = self.find_all().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    /// 删除模型
    pub async fn delete(&self, id: &str) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM ai_models WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::storage::error::RepositoryError::AiModelNotFound {
                id: id.to_string(),
            });
        }

        Ok(())
    }
}
