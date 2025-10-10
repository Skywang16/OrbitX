/*!
 * 最近工作区Repository
 *
 * 处理最近打开的工作区记录的数据访问逻辑
 */

use super::RowMapper;
use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::path::Path;
use std::sync::Arc;

/// 最近工作区条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentWorkspace {
    pub id: i64,
    pub path: String,
    pub last_accessed_at: i64,
}

impl RowMapper<RecentWorkspace> for RecentWorkspace {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            path: row.try_get("path")?,
            last_accessed_at: row.try_get("last_accessed_at")?,
        })
    }
}

/// 最近工作区Repository
pub struct RecentWorkspaceRepository {
    database: Arc<DatabaseManager>,
}

impl RecentWorkspaceRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 添加或更新工作区访问记录
    pub async fn add_or_update(&self, path: &str) -> RepositoryResult<()> {
        let normalized_path = normalize_path(path)?;
        let now = chrono::Utc::now().timestamp();

        let query = r#"
            INSERT INTO recent_workspaces (path, last_accessed_at)
            VALUES (?, ?)
            ON CONFLICT(path) DO UPDATE SET
                last_accessed_at = excluded.last_accessed_at
        "#;

        sqlx::query(query)
            .bind(&normalized_path)
            .bind(now)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 获取最近N个工作区（按last_accessed_at倒序）
    pub async fn get_recent(&self, limit: i64) -> RepositoryResult<Vec<RecentWorkspace>> {
        let query = r#"
            SELECT id, path, last_accessed_at
            FROM recent_workspaces
            ORDER BY last_accessed_at DESC
            LIMIT ?
        "#;

        let rows = sqlx::query(query)
            .bind(limit)
            .fetch_all(self.database.pool())
            .await?;

        rows.iter()
            .map(|row| RecentWorkspace::from_row(row))
            .collect()
    }

    /// 删除指定路径的记录
    pub async fn remove(&self, path: &str) -> RepositoryResult<()> {
        let normalized_path = normalize_path(path)?;
        let query = "DELETE FROM recent_workspaces WHERE path = ?";

        sqlx::query(query)
            .bind(&normalized_path)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 清理超过指定天数未访问的记录
    pub async fn cleanup_old(&self, days: i64) -> RepositoryResult<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        let query = "DELETE FROM recent_workspaces WHERE last_accessed_at < ?";

        let result = sqlx::query(query)
            .bind(cutoff)
            .execute(self.database.pool())
            .await?;

        Ok(result.rows_affected())
    }

    /// 限制记录数量（保留最近访问的 N 条）
    pub async fn limit_records(&self, max_count: i64) -> RepositoryResult<u64> {
        let query = r#"
            DELETE FROM recent_workspaces
            WHERE id NOT IN (
                SELECT id FROM recent_workspaces
                ORDER BY last_accessed_at DESC
                LIMIT ?
            )
        "#;

        let result = sqlx::query(query)
            .bind(max_count)
            .execute(self.database.pool())
            .await?;

        Ok(result.rows_affected())
    }

    /// 维护数据：清理过期记录 + 限制总数
    pub async fn maintain(&self, max_days: i64, max_count: i64) -> RepositoryResult<(u64, u64)> {
        let old_count = self.cleanup_old(max_days).await?;
        let excess_count = self.limit_records(max_count).await?;
        Ok((old_count, excess_count))
    }
}

/// 路径规范化（去除尾部斜杠，展开~等）
fn normalize_path(path: &str) -> RepositoryResult<String> {
    use std::env;

    // 展开 ~ 符号
    let expanded = if path.starts_with("~/") {
        if let Ok(home) = env::var("HOME") {
            path.replacen("~", &home, 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    // 尝试获取规范路径
    let path_obj = Path::new(&expanded);

    // 如果路径存在，则规范化；否则直接使用原路径
    let normalized = if path_obj.exists() {
        path_obj
            .canonicalize()
            .map_err(|e| RepositoryError::Validation {
                reason: format!("Failed to canonicalize path: {}", e),
            })?
            .to_string_lossy()
            .to_string()
    } else {
        // 路径不存在时，只做基本规范化（去除尾部斜杠）
        expanded.trim_end_matches('/').to_string()
    };

    Ok(normalized)
}
