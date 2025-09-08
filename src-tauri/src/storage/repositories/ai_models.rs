/*!
 * AI模型Repository
 *
 * 处理AI模型配置的数据访问逻辑
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, SafeQueryBuilder};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use keyring::Entry;
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
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Claude,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "qwen")]
    Qwen,
    #[serde(rename = "custom")]
    Custom,
}

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIProvider::OpenAI => write!(f, "OpenAI"),
            AIProvider::Claude => write!(f, "Claude"),
            AIProvider::Gemini => write!(f, "Gemini"),
            AIProvider::Qwen => write!(f, "Qwen"),
            AIProvider::Custom => write!(f, "Custom"),
        }
    }
}

impl std::str::FromStr for AIProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AIProvider::OpenAI),
            "anthropic" | "claude" => Ok(AIProvider::Claude),
            "gemini" => Ok(AIProvider::Gemini),
            "qwen" => Ok(AIProvider::Qwen),
            "custom" => Ok(AIProvider::Custom),
            _ => Err(anyhow!("Unknown AI provider: {}", s)),
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
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(ModelType::Chat),
            "embedding" => Ok(ModelType::Embedding),
            _ => Err(anyhow!("Unknown model type: {}", s)),
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
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
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
    user_prefix_prompt: Arc<RwLock<Option<String>>>,
}

impl AIModelRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            database,
            user_prefix_prompt: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn find_all_with_decrypted_keys(&self) -> AppResult<Vec<AIModelConfig>> {
        let (query, params) = SafeQueryBuilder::new("ai_models")
            .select(&[
                "id",
                "name",
                "provider",
                "api_url",
                "api_key_encrypted",
                "model_name",
                "model_type", // 添加缺失的model_type字段
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
                        return Err(anyhow!("Unsupported number type"));
                    }
                }
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => return Err(anyhow!("Unsupported parameter type")),
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

            // 读取API密钥：从系统密钥链读取
            if let Some(stored) = row.try_get::<Option<String>, _>("api_key_encrypted")? {
                if !stored.is_empty() {
                    if stored.starts_with("keychain:") {
                        let key_id = stored.trim_start_matches("keychain:");
                        let entry = match Entry::new("orbitx.ai_models", key_id) {
                            Ok(e) => e,
                            Err(e) => {
                                error!("创建系统密钥链条目失败: {}", e);
                                model.api_key = String::new();
                                models.push(model);
                                continue;
                            }
                        };
                        match entry.get_password() {
                            Ok(secret) => {
                                model.api_key = secret;
                            }
                            Err(e) => {
                                error!("从系统密钥链读取API密钥失败: {}", e);
                                model.api_key = String::new();
                            }
                        }
                    } else {
                        model.api_key = String::new();
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

    pub async fn save_with_encryption(&self, model: &AIModelConfig) -> AppResult<i64> {
        debug!("保存AI模型: {}", model.name);

        // 如果没有提供新的 api_key，则保留原有的 Keychain 标记，不覆盖
        let existing_marker: Option<String> =
            sqlx::query("SELECT api_key_encrypted FROM ai_models WHERE id = ?")
                .bind(&model.id)
                .fetch_optional(self.database.pool())
                .await?
                .and_then(|row| row.try_get::<Option<String>, _>(0).ok())
                .flatten();

        // 将API密钥保存到系统密钥链
        let keychain_marker = if !model.api_key.is_empty() {
            let entry = Entry::new("orbitx.ai_models", &model.id)
                .map_err(|e| anyhow!("创建系统密钥链条目失败: {}", e))?;
            entry
                .set_password(&model.api_key)
                .map_err(|e| anyhow!("保存API密钥到系统密钥链失败: {}", e))?;
            Some(format!("keychain:{}", model.id))
        } else {
            // 未提供新 key，沿用原标记（可能为 None）
            existing_marker
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
                keychain_marker.map(Value::String).unwrap_or(Value::Null),
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
                _ => return Err(anyhow!("Unsupported parameter type")),
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;

        debug!("AI模型保存成功: {}", model.name);
        Ok(result.last_insert_rowid())
    }
}

#[async_trait::async_trait]
impl Repository<AIModelConfig> for AIModelRepository {
    async fn find_by_id(&self, _id: i64) -> AppResult<Option<AIModelConfig>> {
        // AI模型使用字符串ID，这个方法不适用
        Err(anyhow!("AI模型使用字符串ID，请使用find_by_string_id"))
    }

    async fn find_all(&self) -> AppResult<Vec<AIModelConfig>> {
        self.find_all_with_decrypted_keys().await
    }

    async fn save(&self, entity: &AIModelConfig) -> AppResult<i64> {
        self.save_with_encryption(entity).await
    }

    async fn update(&self, entity: &AIModelConfig) -> AppResult<()> {
        self.save_with_encryption(entity).await?;
        Ok(())
    }

    async fn delete(&self, _id: i64) -> AppResult<()> {
        Err(anyhow!("AI模型使用字符串ID，请使用delete_by_string_id"))
    }
}

impl AIModelRepository {
    pub async fn find_by_string_id(&self, id: &str) -> AppResult<Option<AIModelConfig>> {
        let models = self.find_all_with_decrypted_keys().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    pub async fn delete_by_string_id(&self, id: &str) -> AppResult<()> {
        // 从系统密钥链删除对应条目
        if let Ok(entry) = Entry::new("orbitx.ai_models", id) {
            if let Err(e) = entry.delete_password() {
                debug!("从系统密钥链删除API密钥失败（可忽略）: {}", e);
            }
        } else {
            debug!("创建系统密钥链条目失败，跳过删除");
        }

        let result = sqlx::query("DELETE FROM ai_models WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("模型ID不存在: {}", id));
        }

        debug!("AI模型删除成功: {}", id);
        Ok(())
    }

    pub async fn get_user_prefix_prompt(&self) -> AppResult<Option<String>> {
        debug!("从内存缓存获取用户前置提示词");

        let prompt = self.user_prefix_prompt.read().await.clone();
        debug!(
            "用户前置提示词获取成功: {:?}",
            prompt.as_ref().map(|p| p.len())
        );
        Ok(prompt)
    }

    pub async fn set_user_prefix_prompt(&self, prompt: Option<String>) -> AppResult<()> {
        debug!("设置用户前置提示词: {:?}", prompt.as_ref().map(|p| p.len()));

        *self.user_prefix_prompt.write().await = prompt;

        debug!("用户前置提示词设置成功");
        Ok(())
    }
}
