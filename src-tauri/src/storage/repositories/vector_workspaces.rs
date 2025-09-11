/*!
 * 向量工作区Repository
 *
 * 处理工作区索引信息的数据访问逻辑
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

/// 索引状态枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexStatus {
    #[serde(rename = "building")]
    Building,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "error")]
    Error,
}

impl IndexStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "building" => Ok(Self::Building),
            "ready" => Ok(Self::Ready),
            "error" => Ok(Self::Error),
            _ => Err(anyhow!("无效的索引状态: {}", s)),
        }
    }
}

/// 工作区索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIndex {
    pub id: Option<i32>,
    pub workspace_path: String,
    pub name: Option<String>,
    pub status: IndexStatus,
    pub file_count: i32,
    pub index_size_bytes: i64,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceIndex {
    /// 创建新的工作区索引记录
    pub fn new(workspace_path: String, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            workspace_path,
            name,
            status: IndexStatus::Building,
            file_count: 0,
            index_size_bytes: 0,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 更新为准备就绪状态
    pub fn mark_ready(&mut self, file_count: i32, index_size_bytes: i64) {
        self.status = IndexStatus::Ready;
        self.file_count = file_count;
        self.index_size_bytes = index_size_bytes;
        self.error_message = None;
        self.updated_at = Utc::now();
    }

    /// 更新为错误状态
    pub fn mark_error(&mut self, error_message: String) {
        self.status = IndexStatus::Error;
        self.error_message = Some(error_message);
        self.updated_at = Utc::now();
    }

    /// 检查是否为建议状态
    pub fn is_building(&self) -> bool {
        self.status == IndexStatus::Building
    }

    /// 检查是否为准备就绪状态
    pub fn is_ready(&self) -> bool {
        self.status == IndexStatus::Ready
    }

    /// 检查是否为错误状态
    pub fn is_error(&self) -> bool {
        self.status == IndexStatus::Error
    }
}

impl RowMapper<WorkspaceIndex> for WorkspaceIndex {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let status_str: String = row.try_get("status")?;
        let status = IndexStatus::from_str(&status_str).context("解析索引状态失败")?;

        Ok(Self {
            id: Some(row.try_get("id")?),
            workspace_path: row.try_get("workspace_path")?,
            name: row.try_get("name")?,
            status,
            file_count: row.try_get("file_count")?,
            index_size_bytes: row.try_get("index_size_bytes")?,
            error_message: row.try_get("error_message")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 向量工作区Repository
pub struct VectorWorkspaceRepository {
    database: Arc<DatabaseManager>,
}

impl VectorWorkspaceRepository {
    /// 创建新的Repository实例
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 根据工作区路径查找索引
    pub async fn find_by_path(&self, workspace_path: &str) -> AppResult<Option<WorkspaceIndex>> {
        let query_builder =
            SafeQueryBuilder::new("vector_workspaces").where_condition(QueryCondition::eq(
                "workspace_path",
                serde_json::Value::String(workspace_path.to_string()),
            ));

        let (query, params) = query_builder.build_select_all()?;
        let pool = self.database.pool();

        let mut query = sqlx::query(&query);
        for param in params {
            match param {
                serde_json::Value::String(s) => query = query.bind(s),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query = query.bind(i);
                    } else if let Some(f) = n.as_f64() {
                        query = query.bind(f);
                    }
                }
                serde_json::Value::Bool(b) => query = query.bind(b),
                serde_json::Value::Null => query = query.bind(Option::<String>::None),
                _ => {}
            }
        }
        let row = query
            .fetch_optional(pool)
            .await
            .context("查询工作区索引失败")?;

        match row {
            Some(row) => Ok(Some(WorkspaceIndex::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 获取所有工作区索引，按更新时间倒序排列
    pub async fn find_all_ordered(&self) -> AppResult<Vec<WorkspaceIndex>> {
        let query = "SELECT * FROM vector_workspaces ORDER BY updated_at DESC";
        let pool = self.database.pool();

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .context("查询所有工作区索引失败")?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(WorkspaceIndex::from_row(&row)?);
        }

        Ok(workspaces)
    }

    /// 更新索引状态和统计信息
    pub async fn update_status(
        &self,
        id: i32,
        status: IndexStatus,
        file_count: Option<i32>,
        index_size_bytes: Option<i64>,
        error_message: Option<String>,
    ) -> AppResult<()> {
        let query = "UPDATE vector_workspaces SET status = ?, file_count = ?, index_size_bytes = ?, error_message = ?, updated_at = ? WHERE id = ?";
        let pool = self.database.pool();

        sqlx::query(query)
            .bind(status.as_str())
            .bind(file_count)
            .bind(index_size_bytes)
            .bind(error_message)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await
            .context("更新工作区索引状态失败")?;

        Ok(())
    }

    /// 根据状态查询工作区
    pub async fn find_by_status(&self, status: IndexStatus) -> AppResult<Vec<WorkspaceIndex>> {
        let query_builder =
            SafeQueryBuilder::new("vector_workspaces").where_condition(QueryCondition::eq(
                "status",
                serde_json::Value::String(status.as_str().to_string()),
            ));

        let (query, params) = query_builder.build_select_all()?;
        let pool = self.database.pool();

        let mut sqlx_query = sqlx::query(&query);
        for param in params {
            match param {
                serde_json::Value::String(s) => sqlx_query = sqlx_query.bind(s),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        sqlx_query = sqlx_query.bind(i);
                    } else if let Some(f) = n.as_f64() {
                        sqlx_query = sqlx_query.bind(f);
                    }
                }
                serde_json::Value::Bool(b) => sqlx_query = sqlx_query.bind(b),
                serde_json::Value::Null => sqlx_query = sqlx_query.bind(Option::<String>::None),
                _ => {}
            }
        }

        let rows = sqlx_query
            .fetch_all(pool)
            .await
            .context("根据状态查询工作区索引失败")?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(WorkspaceIndex::from_row(&row)?);
        }

        Ok(workspaces)
    }

    /// 检查工作区路径是否已存在
    pub async fn path_exists(&self, workspace_path: &str) -> AppResult<bool> {
        let query = "SELECT COUNT(*) as count FROM vector_workspaces WHERE workspace_path = ?";
        let pool = self.database.pool();

        let row = sqlx::query(query)
            .bind(workspace_path)
            .fetch_one(pool)
            .await
            .context("检查工作区路径是否存在失败")?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }
}

#[async_trait::async_trait]
impl Repository<WorkspaceIndex> for VectorWorkspaceRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<WorkspaceIndex>> {
        let query_builder =
            SafeQueryBuilder::new("vector_workspaces").where_condition(QueryCondition::eq(
                "id",
                serde_json::Value::Number(serde_json::Number::from(id)),
            ));

        let (query, params) = query_builder.build_select_all()?;
        let pool = self.database.pool();

        let mut sqlx_query = sqlx::query(&query);
        for param in params {
            match param {
                serde_json::Value::String(s) => sqlx_query = sqlx_query.bind(s),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        sqlx_query = sqlx_query.bind(i);
                    } else if let Some(f) = n.as_f64() {
                        sqlx_query = sqlx_query.bind(f);
                    }
                }
                serde_json::Value::Bool(b) => sqlx_query = sqlx_query.bind(b),
                serde_json::Value::Null => sqlx_query = sqlx_query.bind(Option::<String>::None),
                _ => {}
            }
        }

        let row = sqlx_query
            .fetch_optional(pool)
            .await
            .context("根据ID查询工作区索引失败")?;

        match row {
            Some(row) => Ok(Some(WorkspaceIndex::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<WorkspaceIndex>> {
        self.find_all_ordered().await
    }

    async fn save(&self, entity: &WorkspaceIndex) -> AppResult<i64> {
        let insert_builder = InsertBuilder::new("vector_workspaces")
            .add_field("workspace_path", &entity.workspace_path)
            .add_field_opt("name", entity.name.as_ref())
            .add_field("status", &entity.status)
            .add_field("file_count", &entity.file_count)
            .add_field("index_size_bytes", &entity.index_size_bytes)
            .add_field_opt("error_message", entity.error_message.as_ref())
            .add_field("created_at", &entity.created_at)
            .add_field("updated_at", &entity.updated_at);

        let (query, values) = insert_builder.build()?;
        let pool = self.database.pool();

        let mut sqlx_query = sqlx::query(&query);
        for value in &values {
            match value {
                serde_json::Value::String(s) => sqlx_query = sqlx_query.bind(s),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        sqlx_query = sqlx_query.bind(i);
                    } else if let Some(f) = n.as_f64() {
                        sqlx_query = sqlx_query.bind(f);
                    }
                }
                serde_json::Value::Bool(b) => sqlx_query = sqlx_query.bind(*b),
                serde_json::Value::Null => sqlx_query = sqlx_query.bind(Option::<String>::None),
                _ => {
                    let s = value.to_string();
                    sqlx_query = sqlx_query.bind(s);
                }
            }
        }

        let result = sqlx_query
            .execute(pool)
            .await
            .context("保存工作区索引失败")?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, entity: &WorkspaceIndex) -> AppResult<()> {
        let id = entity.id.ok_or_else(|| anyhow!("工作区索引ID不能为空"))?;

        let query = "UPDATE vector_workspaces SET workspace_path = ?, name = ?, status = ?, file_count = ?, index_size_bytes = ?, error_message = ?, updated_at = ? WHERE id = ?";
        let pool = self.database.pool();

        sqlx::query(query)
            .bind(&entity.workspace_path)
            .bind(&entity.name)
            .bind(entity.status.as_str())
            .bind(entity.file_count)
            .bind(entity.index_size_bytes)
            .bind(&entity.error_message)
            .bind(&entity.updated_at)
            .bind(id)
            .execute(pool)
            .await
            .context("更新工作区索引失败")?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let query = "DELETE FROM vector_workspaces WHERE id = ?";
        let pool = self.database.pool();

        sqlx::query(query)
            .bind(id)
            .execute(pool)
            .await
            .context("删除工作区索引失败")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_status_serialization() {
        let status = IndexStatus::Building;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"building\"");

        let deserialized: IndexStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, IndexStatus::Building);
    }

    #[test]
    fn test_workspace_index_creation() {
        let workspace = WorkspaceIndex::new(
            "/path/to/workspace".to_string(),
            Some("Test Workspace".to_string()),
        );

        assert_eq!(workspace.workspace_path, "/path/to/workspace");
        assert_eq!(workspace.name, Some("Test Workspace".to_string()));
        assert_eq!(workspace.status, IndexStatus::Building);
        assert_eq!(workspace.file_count, 0);
        assert_eq!(workspace.index_size_bytes, 0);
        assert!(workspace.error_message.is_none());
    }

    #[test]
    fn test_workspace_index_state_transitions() {
        let mut workspace = WorkspaceIndex::new("/path/to/workspace".to_string(), None);

        // Test marking as ready
        workspace.mark_ready(100, 1024);
        assert!(workspace.is_ready());
        assert_eq!(workspace.file_count, 100);
        assert_eq!(workspace.index_size_bytes, 1024);
        assert!(workspace.error_message.is_none());

        // Test marking as error
        workspace.mark_error("索引构建失败".to_string());
        assert!(workspace.is_error());
        assert_eq!(workspace.error_message, Some("索引构建失败".to_string()));
    }

    #[test]
    fn test_index_status_from_str() {
        assert_eq!(
            IndexStatus::from_str("building").unwrap(),
            IndexStatus::Building
        );
        assert_eq!(IndexStatus::from_str("ready").unwrap(), IndexStatus::Ready);
        assert_eq!(IndexStatus::from_str("error").unwrap(), IndexStatus::Error);
        assert!(IndexStatus::from_str("invalid").is_err());
    }
}
