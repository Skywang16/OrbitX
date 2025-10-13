/*!
 * AI模型Repository
 *
 * 处理AI模型配置的数据访问逻辑
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use crate::storage::query::{InsertBuilder, SafeQueryBuilder};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

fn default_enabled() -> bool {
    true
}

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

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIProvider::Anthropic => write!(f, "Anthropic"),
            AIProvider::OpenAiCompatible => write!(f, "OpenAI Compatible"),
        }
    }
}

impl std::str::FromStr for AIProvider {
    type Err = RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(AIProvider::Anthropic),
            "openai_compatible" => Ok(AIProvider::OpenAiCompatible),
            _ => Err(RepositoryError::Validation {
                reason: format!("Unknown AI provider: {}", s),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    #[serde(rename = "chat")]
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
    type Err = RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(ModelType::Chat),
            "embedding" => Ok(ModelType::Embedding),
            _ => Err(RepositoryError::Validation {
                reason: format!("Unknown model type: {}", s),
            }),
        }
    }
}

impl Default for ModelType {
    fn default() -> Self {
        ModelType::Chat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    pub name: String,
    pub provider: AIProvider,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub model_type: ModelType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
}

impl AIModelConfig {
    pub fn new(
        name: String,
        provider: AIProvider,
        api_url: String,
        api_key: String,
        model: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            provider,
            api_url,
            api_key,
            model,
            model_type: ModelType::Chat, // 默认为聊天模型
            enabled: true,
            options: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_model_type(
        name: String,
        provider: AIProvider,
        api_url: String,
        api_key: String,
        model: String,
        model_type: ModelType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            provider,
            api_url,
            api_key,
            model,
            model_type,
            enabled: true,
            options: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl RowMapper<AIModelConfig> for AIModelConfig {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<Self> {
        let provider_str: String = row.try_get("provider")?;
        let provider = provider_str.parse()?;

        let model_type_str: String = row.try_get("model_type")?;
        let model_type = model_type_str.parse()?;

        let options = if let Some(config_json) = row.try_get::<Option<String>, _>("config_json")? {
            serde_json::from_str(&config_json).ok()
        } else {
            None
        };

        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            provider,
            api_url: row.try_get("api_url")?,
            api_key: String::new(), // 加密的API密钥需要单独解密
            model: row.try_get("model_name")?,
            model_type,
            enabled: row.try_get("enabled")?,
            options,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

pub struct AIModelRepository {
    database: Arc<DatabaseManager>,
    user_rules: Arc<RwLock<Option<String>>>,
    project_rules: Arc<RwLock<Option<String>>>,
}

impl AIModelRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            database,
            user_rules: Arc::new(RwLock::new(None)),
            project_rules: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn find_all_with_decrypted_keys(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        let (query, params) = SafeQueryBuilder::new("ai_models")
            .select(&[
                "id",
                "name",
                "provider",
                "api_url",
                "api_key_encrypted",
                "model_name",
                "model_type",
                "enabled",
                "config_json",
                "created_at",
                "updated_at",
            ])
            .order_by(crate::storage::query::QueryOrder::Asc(
                "created_at".to_string(),
            ))
            .build()?;

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query_builder.bind(f)
                    } else {
                        return Err(RepositoryError::UnsupportedNumberType);
                    }
                }
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "ai_models parameter",
                    ))
                }
            };
        }

        let rows = query_builder.fetch_all(self.database.pool()).await?;

        let mut models = Vec::new();

        for row in rows {
            let mut model = match AIModelConfig::from_row(&row) {
                Ok(m) => m,
                Err(e) => {
                    error!("解析数据失败: {}", e);
                    continue;
                }
            };

            // 从数据库读取加密的API密钥并解密
            if let Some(encrypted_base64) = row.try_get::<Option<String>, _>("api_key_encrypted")? {
                if !encrypted_base64.is_empty() {
                    match BASE64.decode(&encrypted_base64) {
                        Ok(encrypted_bytes) => {
                            match self.database.decrypt_data(&encrypted_bytes).await {
                                Ok(decrypted) => {
                                    model.api_key = decrypted;
                                }
                                Err(e) => {
                                    error!("解密API密钥失败 (model_id={}): {}", model.id, e);
                                    model.api_key = String::new();
                                }
                            }
                        }
                        Err(e) => {
                            error!("Base64解码失败 (model_id={}): {}", model.id, e);
                            model.api_key = String::new();
                        }
                    }
                } else {
                    model.api_key = String::new();
                }
            } else {
                model.api_key = String::new();
            }

            models.push(model);
        }
        Ok(models)
    }

    pub async fn save_with_encryption(&self, model: &AIModelConfig) -> RepositoryResult<i64> {
        debug!("保存AI模型: {}", model.name);

        // 加密API密钥
        let encrypted_key = if !model.api_key.is_empty() {
            let encrypted_bytes = self.database.encrypt_data(&model.api_key).await.map_err(|e| {
                RepositoryError::internal(format!(
                    "加密API密钥失败 (model_id={}): {}",
                    model.id, e
                ))
            })?;
            Some(BASE64.encode(&encrypted_bytes))
        } else {
            // 如果没有提供新密钥，保留现有的加密密钥
            sqlx::query("SELECT api_key_encrypted FROM ai_models WHERE id = ?")
                .bind(&model.id)
                .fetch_optional(self.database.pool())
                .await?
                .and_then(|row| row.try_get::<Option<String>, _>(0).ok())
                .flatten()
        };

        let config_json = model
            .options
            .as_ref()
            .map(|opts| serde_json::to_string(opts).unwrap_or_default());

        let (query, params) = InsertBuilder::new("ai_models")
            .on_conflict_replace()
            .set("id", Value::String(model.id.clone()))
            .set("name", Value::String(model.name.clone()))
            .set("provider", Value::String(model.provider.to_string()))
            .set("api_url", Value::String(model.api_url.clone()))
            .set(
                "api_key_encrypted",
                encrypted_key.map(Value::String).unwrap_or(Value::Null),
            )
            .set("model_name", Value::String(model.model.clone()))
            .set("model_type", Value::String(model.model_type.to_string()))
            .set("enabled", Value::Bool(model.enabled))
            .set(
                "config_json",
                config_json.map(Value::String).unwrap_or(Value::Null),
            )
            .set("created_at", Value::String(model.created_at.to_rfc3339()))
            .set("updated_at", Value::String(model.updated_at.to_rfc3339()))
            .build()?;

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "ai_models parameter",
                    ))
                }
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;

        debug!("AI模型保存成功: {}", model.name);
        Ok(result.last_insert_rowid())
    }
}

#[async_trait::async_trait]
impl Repository<AIModelConfig> for AIModelRepository {
    async fn find_by_id(&self, _id: i64) -> RepositoryResult<Option<AIModelConfig>> {
        // AI模型使用字符串ID，这个方法不适用
        Err(RepositoryError::AiModelRequiresStringId {
            recommended: "find_by_string_id",
        })
    }

    async fn find_all(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        self.find_all_with_decrypted_keys().await
    }

    async fn save(&self, entity: &AIModelConfig) -> RepositoryResult<i64> {
        self.save_with_encryption(entity).await
    }

    async fn update(&self, entity: &AIModelConfig) -> RepositoryResult<()> {
        self.save_with_encryption(entity).await?;
        Ok(())
    }

    async fn delete(&self, _id: i64) -> RepositoryResult<()> {
        Err(RepositoryError::AiModelRequiresStringId {
            recommended: "delete_by_string_id",
        })
    }
}

impl AIModelRepository {
    pub async fn find_by_string_id(&self, id: &str) -> RepositoryResult<Option<AIModelConfig>> {
        let models = self.find_all_with_decrypted_keys().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    pub async fn delete_by_string_id(&self, id: &str) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM ai_models WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::AiModelNotFound { id: id.to_string() });
        }

        debug!("AI模型删除成功: {}", id);
        Ok(())
    }

    pub async fn get_user_rules(&self) -> RepositoryResult<Option<String>> {
        debug!("从内存缓存获取用户规则");

        let rules = self.user_rules.read().await.clone();
        debug!(
            "用户规则获取成功: {:?}",
            rules.as_ref().map(|r| r.len())
        );
        Ok(rules)
    }

    pub async fn set_user_rules(&self, rules: Option<String>) -> RepositoryResult<()> {
        debug!("设置用户规则: {:?}", rules.as_ref().map(|r| r.len()));

        *self.user_rules.write().await = rules;

        debug!("用户规则设置成功");
        Ok(())
    }

    pub async fn get_project_rules(&self) -> RepositoryResult<Option<String>> {
        let rules = self.project_rules.read().await.clone();
        Ok(rules)
    }

    pub async fn set_project_rules(&self, rules: Option<String>) -> RepositoryResult<()> {
        debug!("设置项目规则: {:?}", rules);

        *self.project_rules.write().await = rules;

        debug!("项目规则设置成功");
        Ok(())
    }
}
