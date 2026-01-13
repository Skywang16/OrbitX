/*!
 * AI 功能配置 - 直接使用 sqlx，去除虚假抽象
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// AI 功能配置实体
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

impl AIFeatureConfig {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<Self> {
        Ok(Self {
            feature_name: row.try_get("feature_name")?,
            enabled: row.try_get("enabled")?,
            config_json: row.try_get("config_json")?,
            created_at: DateTime::parse_from_rfc3339(&row.try_get::<String, _>("created_at")?)
                .map_err(|e| {
                    RepositoryError::internal(format!(
                        "Failed to parse created_at timestamp: {e}"
                    ))
                })?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.try_get::<String, _>("updated_at")?)
                .map_err(|e| {
                    RepositoryError::internal(format!(
                        "Failed to parse updated_at timestamp: {e}"
                    ))
                })?
                .with_timezone(&Utc),
        })
    }
}

/// AIFeatures 数据访问
pub struct AIFeatures<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AIFeatures<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    /// 根据功能名称查找配置
    pub async fn find_by_feature_name(
        &self,
        feature_name: &str,
    ) -> RepositoryResult<Option<AIFeatureConfig>> {
        let row_opt = sqlx::query(
            r#"
            SELECT feature_name, enabled, config_json, created_at, updated_at
            FROM ai_features WHERE feature_name = ? LIMIT 1
            "#,
        )
        .bind(feature_name)
        .fetch_optional(self.db.pool())
        .await?;

        row_opt
            .map(|row| AIFeatureConfig::from_row(&row))
            .transpose()
    }

    /// 保存或更新功能配置
    pub async fn save_or_update(&self, config: &AIFeatureConfig) -> RepositoryResult<()> {
        let updated = AIFeatureConfig {
            updated_at: Utc::now(),
            ..config.clone()
        };

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO ai_features
            (feature_name, enabled, config_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&updated.feature_name)
        .bind(updated.enabled)
        .bind(&updated.config_json)
        .bind(updated.created_at.to_rfc3339())
        .bind(updated.updated_at.to_rfc3339())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// 删除功能配置
    pub async fn delete_by_feature_name(&self, feature_name: &str) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM ai_features WHERE feature_name = ?")
            .bind(feature_name)
            .execute(self.db.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::AiFeatureNotFound {
                name: feature_name.to_string(),
            });
        }
        Ok(())
    }

    /// 获取所有功能配置
    pub async fn find_all_features(&self) -> RepositoryResult<Vec<AIFeatureConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT feature_name, enabled, config_json, created_at, updated_at
            FROM ai_features
            ORDER BY feature_name ASC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        rows.into_iter()
            .map(|row| AIFeatureConfig::from_row(&row))
            .collect()
    }
}
