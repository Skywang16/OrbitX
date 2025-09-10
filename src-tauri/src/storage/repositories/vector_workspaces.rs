/*!
 * 向量索引工作区数据访问层
 *
 * 提供工作区的CRUD操作和状态管理
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

/// 工作区状态枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatus {
    Uninitialized,
    Building,
    Ready,
    Error,
}

impl std::fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceStatus::Uninitialized => write!(f, "uninitialized"),
            WorkspaceStatus::Building => write!(f, "building"),
            WorkspaceStatus::Ready => write!(f, "ready"),
            WorkspaceStatus::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for WorkspaceStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uninitialized" => Ok(WorkspaceStatus::Uninitialized),
            "building" => Ok(WorkspaceStatus::Building),
            "ready" => Ok(WorkspaceStatus::Ready),
            "error" => Ok(WorkspaceStatus::Error),
            _ => Err(anyhow!("Invalid workspace status: {}", s)),
        }
    }
}

/// 向量索引工作区实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorWorkspace {
    pub id: String,
    pub path: String,
    pub name: String,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VectorWorkspace {
    /// 创建新的工作区实例
    pub fn new(id: String, path: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            path,
            name,
            status: WorkspaceStatus::Uninitialized,
            created_at: now,
            updated_at: now,
        }
    }

    /// 检查工作区是否对前端可见（状态为Ready）
    pub fn is_visible(&self) -> bool {
        matches!(self.status, WorkspaceStatus::Ready)
    }

    /// 更新工作区状态
    pub fn set_status(&mut self, status: WorkspaceStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}

impl RowMapper<VectorWorkspace> for VectorWorkspace {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let status_str: String = row.try_get("status")?;
        let status = status_str.parse()?;

        Ok(Self {
            id: row.try_get("id")?,
            path: row.try_get("path")?,
            name: row.try_get("name")?,
            status,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 向量索引工作区数据访问层
pub struct VectorWorkspaceRepository {
    database: Arc<DatabaseManager>,
}

impl VectorWorkspaceRepository {
    /// 创建新的工作区仓库实例
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 根据路径查找工作区
    pub async fn find_by_path(&self, path: &str) -> AppResult<Option<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces WHERE path = ?";
        let row = sqlx::query(sql)
            .bind(path)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(VectorWorkspace::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 查找所有可见的工作区（状态为Ready）
    pub async fn find_visible_workspaces(&self) -> AppResult<Vec<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces WHERE status = 'ready' ORDER BY updated_at DESC";
        let rows = sqlx::query(sql)
            .fetch_all(self.database.pool())
            .await?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(VectorWorkspace::from_row(&row)?);
        }
        Ok(workspaces)
    }

    /// 根据状态查找工作区
    pub async fn find_by_status(&self, status: WorkspaceStatus) -> AppResult<Vec<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces WHERE status = ? ORDER BY updated_at DESC";
        let rows = sqlx::query(sql)
            .bind(status.to_string())
            .fetch_all(self.database.pool())
            .await?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(VectorWorkspace::from_row(&row)?);
        }
        Ok(workspaces)
    }

    /// 根据字符串ID查找工作区
    pub async fn find_by_string_id(&self, id: &str) -> AppResult<Option<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(VectorWorkspace::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 更新工作区状态
    pub async fn update_status(&self, id: &str, status: WorkspaceStatus) -> AppResult<()> {
        let sql = r#"
            UPDATE vector_workspaces 
            SET status = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
        "#;

        let result = sqlx::query(sql)
            .bind(status.to_string())
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Workspace not found: {}", id));
        }

        Ok(())
    }

    /// 删除工作区（按字符串ID）
    pub async fn delete_by_string_id(&self, id: &str) -> AppResult<()> {
        let sql = "DELETE FROM vector_workspaces WHERE id = ?";
        let result = sqlx::query(sql)
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Workspace not found: {}", id));
        }

        Ok(())
    }

    /// 删除工作区（按路径）
    pub async fn delete_by_path(&self, path: &str) -> AppResult<()> {
        let sql = "DELETE FROM vector_workspaces WHERE path = ?";
        let result = sqlx::query(sql)
            .bind(path)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Workspace not found: {}", path));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Repository<VectorWorkspace> for VectorWorkspaceRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id.to_string())
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(VectorWorkspace::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<VectorWorkspace>> {
        let sql = "SELECT * FROM vector_workspaces ORDER BY updated_at DESC";
        let rows = sqlx::query(sql)
            .fetch_all(self.database.pool())
            .await?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(VectorWorkspace::from_row(&row)?);
        }
        Ok(workspaces)
    }

    async fn save(&self, workspace: &VectorWorkspace) -> AppResult<i64> {
        let sql = r#"
            INSERT OR REPLACE INTO vector_workspaces (
                id, path, name, status, updated_at
            ) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        "#;

        let result = sqlx::query(sql)
            .bind(&workspace.id)
            .bind(&workspace.path)
            .bind(&workspace.name)
            .bind(workspace.status.to_string())
            .execute(self.database.pool())
            .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, workspace: &VectorWorkspace) -> AppResult<()> {
        self.save(workspace).await?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let sql = "DELETE FROM vector_workspaces WHERE id = ?";
        let result = sqlx::query(sql)
            .bind(id.to_string())
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Workspace not found: {}", id));
        }

        Ok(())
    }
}