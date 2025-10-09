/*!
 * AI功能配置Repository
 *
 * 处理AI功能配置的数据访问逻辑，支持单一功能配置的存储和检索。
 * 用于存储如聊天、向量索引等AI功能的配置信息。
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use tracing::debug;

/// AI功能配置实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIFeatureConfig {
    /// 功能名称（主键）
    pub feature_name: String,
    /// 功能是否启用
    pub enabled: bool,
    /// 功能配置JSON
    pub config_json: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl AIFeatureConfig {
    /// 创建新的功能配置
    pub fn new(feature_name: String, enabled: bool, config_json: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            feature_name,
            enabled,
            config_json,
            created_at: now,
            updated_at: now,
        }
    }

    /// 从配置对象创建功能配置
    pub fn from_config<T: Serialize>(
        feature_name: String,
        enabled: bool,
        config: &T,
    ) -> RepositoryResult<Self> {
        let config_json = serde_json::to_string(config)?;

        Ok(Self::new(feature_name, enabled, Some(config_json)))
    }

    /// 解析配置JSON为指定类型
    pub fn parse_config<T: for<'de> Deserialize<'de>>(&self) -> RepositoryResult<Option<T>> {
        match &self.config_json {
            Some(json) => {
                let config = serde_json::from_str(json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }
}

impl RowMapper<AIFeatureConfig> for AIFeatureConfig {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<Self> {
        Ok(Self {
            feature_name: row.try_get("feature_name")?,
            enabled: row.try_get("enabled")?,
            config_json: row.try_get("config_json")?,
            created_at: {
                let timestamp: String = row.try_get("created_at")?;
                DateTime::parse_from_rfc3339(&timestamp)
                    .map_err(|e| RepositoryError::internal(format!(
                        "Failed to parse created_at timestamp: {}",
                        e
                    )))?
                    .with_timezone(&Utc)
            },
            updated_at: {
                let timestamp: String = row.try_get("updated_at")?;
                DateTime::parse_from_rfc3339(&timestamp)
                    .map_err(|e| RepositoryError::internal(format!(
                        "Failed to parse updated_at timestamp: {}",
                        e
                    )))?
                    .with_timezone(&Utc)
            },
        })
    }
}

/// AI功能配置Repository
pub struct AIFeaturesRepository {
    database: Arc<DatabaseManager>,
}

impl AIFeaturesRepository {
    /// 创建新的AI功能配置Repository
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 根据功能名称查找配置
    pub async fn find_by_feature_name(
        &self,
        feature_name: &str,
    ) -> RepositoryResult<Option<AIFeatureConfig>> {
        debug!("查找AI功能配置: {}", feature_name);

        let (query, params) = SafeQueryBuilder::new("ai_features")
            .select(&[
                "feature_name",
                "enabled",
                "config_json",
                "created_at",
                "updated_at",
            ])
            .where_condition(QueryCondition::Eq(
                "feature_name".to_string(),
                Value::String(feature_name.to_string()),
            ))
            .build()?;

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "ai_features parameter",
                    ))
                }
            };
        }

        let row_opt = query_builder.fetch_optional(self.database.pool()).await?;

        match row_opt {
            Some(row) => {
                let config = AIFeatureConfig::from_row(&row)?;
                debug!("找到AI功能配置: {}", feature_name);
                Ok(Some(config))
            }
            None => {
                debug!("未找到AI功能配置: {}", feature_name);
                Ok(None)
            }
        }
    }

    /// 保存或更新功能配置
    pub async fn save_or_update(&self, config: &AIFeatureConfig) -> RepositoryResult<()> {
        debug!("保存AI功能配置: {}", config.feature_name);

        let updated_config = AIFeatureConfig {
            updated_at: Utc::now(),
            ..config.clone()
        };

        let (query, params) = InsertBuilder::new("ai_features")
            .on_conflict_replace()
            .set(
                "feature_name",
                Value::String(updated_config.feature_name.clone()),
            )
            .set("enabled", Value::Bool(updated_config.enabled))
            .set(
                "config_json",
                updated_config
                    .config_json
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            )
            .set(
                "created_at",
                Value::String(updated_config.created_at.to_rfc3339()),
            )
            .set(
                "updated_at",
                Value::String(updated_config.updated_at.to_rfc3339()),
            )
            .build()?;

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "ai_features parameter",
                    ))
                }
            };
        }

        query_builder.execute(self.database.pool()).await?;

        debug!("AI功能配置保存成功: {}", updated_config.feature_name);
        Ok(())
    }

    /// 删除功能配置
    pub async fn delete_by_feature_name(&self, feature_name: &str) -> RepositoryResult<()> {
        debug!("删除AI功能配置: {}", feature_name);

        let result = sqlx::query("DELETE FROM ai_features WHERE feature_name = ?")
            .bind(feature_name)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::AiFeatureNotFound {
                name: feature_name.to_string(),
            });
        }

        debug!("AI功能配置删除成功: {}", feature_name);
        Ok(())
    }

    /// 获取所有功能配置
    pub async fn find_all_features(&self) -> RepositoryResult<Vec<AIFeatureConfig>> {
        debug!("获取所有AI功能配置");

        let (query, params) = SafeQueryBuilder::new("ai_features")
            .select(&[
                "feature_name",
                "enabled",
                "config_json",
                "created_at",
                "updated_at",
            ])
            .order_by(crate::storage::query::QueryOrder::Asc(
                "feature_name".to_string(),
            ))
            .build()?;

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "ai_features parameter",
                    ))
                }
            };
        }

        let rows = query_builder.fetch_all(self.database.pool()).await?;
        let mut configs = Vec::new();

        for row in rows {
            let config = AIFeatureConfig::from_row(&row)?;
            configs.push(config);
        }

        debug!("获取到 {} 个AI功能配置", configs.len());
        Ok(configs)
    }
}

#[async_trait::async_trait]
impl Repository<AIFeatureConfig> for AIFeaturesRepository {
    async fn find_by_id(&self, _id: i64) -> RepositoryResult<Option<AIFeatureConfig>> {
        Err(RepositoryError::AiFeatureRequiresStringId {
            recommended: "find_by_feature_name",
        })
    }

    async fn find_all(&self) -> RepositoryResult<Vec<AIFeatureConfig>> {
        self.find_all_features().await
    }

    async fn save(&self, entity: &AIFeatureConfig) -> RepositoryResult<i64> {
        self.save_or_update(entity).await?;
        Ok(0) // AI功能配置不使用数字ID
    }

    async fn update(&self, entity: &AIFeatureConfig) -> RepositoryResult<()> {
        self.save_or_update(entity).await
    }

    async fn delete(&self, _id: i64) -> RepositoryResult<()> {
        Err(RepositoryError::AiFeatureRequiresStringId {
            recommended: "delete_by_feature_name",
        })
    }
}
