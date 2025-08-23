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
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

/// 简单的内存缓存存储用户前置提示词
static USER_PREFIX_PROMPT: Mutex<Option<String>> = Mutex::new(None);

/// 默认启用状态
fn default_enabled() -> bool {
    true
}

/// 默认时间戳
fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// AI提供商类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AIProvider {
    OpenAI,
    Claude,
    Custom,
}

impl std::fmt::Display for AIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIProvider::OpenAI => write!(f, "OpenAI"),
            AIProvider::Claude => write!(f, "Claude"),
            AIProvider::Custom => write!(f, "Custom"),
        }
    }
}

impl std::str::FromStr for AIProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OpenAI" => Ok(AIProvider::OpenAI),
            "Claude" => Ok(AIProvider::Claude),
            "Custom" => Ok(AIProvider::Custom),
            _ => Err(anyhow!("Unknown AI provider: {}", s)),
        }
    }
}

/// AI模型配置
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
    pub is_default: Option<bool>,
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
            is_default: Some(false),
            enabled: true,
            options: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_default(&self) -> bool {
        self.is_default.unwrap_or(false)
    }

    pub fn set_default(&mut self, is_default: bool) {
        self.is_default = Some(is_default);
        self.updated_at = Utc::now();
    }
}

impl RowMapper<AIModelConfig> for AIModelConfig {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let provider_str: String = row.try_get("provider")?;
        let provider = provider_str.parse()?;

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
            is_default: Some(row.try_get("is_default")?),
            enabled: row.try_get("enabled")?,
            options,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// AI模型Repository
pub struct AIModelRepository {
    database: Arc<DatabaseManager>,
}

impl AIModelRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 获取所有AI模型
    pub async fn find_all_with_decrypted_keys(&self) -> AppResult<Vec<AIModelConfig>> {
        let (query, params) = SafeQueryBuilder::new("ai_models")
            .select(&[
                "id",
                "name",
                "provider",
                "api_url",
                "api_key_encrypted",
                "model_name",
                "is_default",
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

        let encryption_manager_guard = self.database.encryption_manager();
        let encryption_manager = encryption_manager_guard.read().await;
        let mut models = Vec::new();

        for row in rows {
            let mut model = AIModelConfig::from_row(&row)?;

            // 解密API密钥
            if let Some(encrypted_base64) = row.try_get::<Option<String>, _>("api_key_encrypted")? {
                if !encrypted_base64.is_empty() {
                    match base64::engine::general_purpose::STANDARD.decode(&encrypted_base64) {
                        Ok(encrypted_data) => {
                            match encryption_manager.decrypt_data(&encrypted_data) {
                                Ok(decrypted_key) => {
                                    model.api_key = decrypted_key;
                                }
                                Err(e) => {
                                    error!("解密API密钥失败: {}", e);
                                    model.api_key = String::new();
                                }
                            }
                        }
                        Err(e) => {
                            error!("Base64解码失败: {}", e);
                            model.api_key = String::new();
                        }
                    }
                }
            }

            models.push(model);
        }

        Ok(models)
    }

    /// 获取默认AI模型
    pub async fn find_default(&self) -> AppResult<Option<AIModelConfig>> {
        let models = self.find_all_with_decrypted_keys().await?;
        Ok(models.into_iter().find(|m| m.is_default()))
    }

    /// 设置默认AI模型
    pub async fn set_default(&self, model_id: &str) -> AppResult<()> {
        let mut tx = self.database.pool().begin().await?;

        // 清除所有默认标记
        sqlx::query("UPDATE ai_models SET is_default = FALSE")
            .execute(&mut *tx)
            .await?;

        // 设置新的默认模型
        let result = sqlx::query("UPDATE ai_models SET is_default = TRUE WHERE id = ?")
            .bind(model_id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("模型ID不存在: {}", model_id));
        }

        tx.commit().await?;
        Ok(())
    }

    /// 保存AI模型（加密API密钥）
    pub async fn save_with_encryption(&self, model: &AIModelConfig) -> AppResult<i64> {
        debug!("保存AI模型: {}", model.name);

        // 加密API密钥
        let encrypted_api_key = if !model.api_key.is_empty() {
            let encryption_manager_guard = self.database.encryption_manager();
            let encryption_manager = encryption_manager_guard.read().await;
            let encrypted_data = encryption_manager.encrypt_data(&model.api_key)?;
            drop(encryption_manager); // 显式释放锁
            Some(encrypted_data)
        } else {
            None
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
                encrypted_api_key
                    .map(|data| {
                        Value::String(base64::engine::general_purpose::STANDARD.encode(data))
                    })
                    .unwrap_or(Value::Null),
            )
            .set("model_name", Value::String(model.model.clone()))
            .set("is_default", Value::Bool(model.is_default()))
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

        info!("AI模型保存成功: {}", model.name);
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
    /// 根据字符串ID查找
    pub async fn find_by_string_id(&self, id: &str) -> AppResult<Option<AIModelConfig>> {
        let models = self.find_all_with_decrypted_keys().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    /// 根据字符串ID删除
    pub async fn delete_by_string_id(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM ai_models WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("模型ID不存在: {}", id));
        }

        info!("AI模型删除成功: {}", id);
        Ok(())
    }

    /// 获取用户前置提示词
    pub async fn get_user_prefix_prompt(&self) -> AppResult<Option<String>> {
        debug!("从内存缓存获取用户前置提示词");

        let prompt = USER_PREFIX_PROMPT.lock().unwrap().clone();
        debug!(
            "用户前置提示词获取成功: {:?}",
            prompt.as_ref().map(|p| p.len())
        );
        Ok(prompt)
    }

    /// 设置用户前置提示词
    pub async fn set_user_prefix_prompt(&self, prompt: Option<String>) -> AppResult<()> {
        debug!("设置用户前置提示词: {:?}", prompt.as_ref().map(|p| p.len()));

        *USER_PREFIX_PROMPT.lock().unwrap() = prompt;

        info!("用户前置提示词设置成功");
        Ok(())
    }
}
