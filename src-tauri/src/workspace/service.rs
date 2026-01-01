use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::Serialize;
use sqlx::{self, Row};
use tokio::task;

use crate::agent::persistence::AgentPersistence;
use crate::storage::DatabaseManager;

/// 未分组工作区的特殊路径标识
pub const UNGROUPED_WORKSPACE_PATH: &str = "__ungrouped__";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub path: String,
    pub display_name: Option<String>,
    pub active_session_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: i64,
    pub workspace_path: String,
    pub title: Option<String>,
    pub message_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessageRecord {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: Option<String>,
    pub steps_json: Option<String>,
    pub images_json: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
}

pub struct WorkspaceService {
    database: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
}

impl WorkspaceService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let persistence = Arc::new(AgentPersistence::new(Arc::clone(&database)));
        Self {
            database,
            agent_persistence: persistence,
        }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    fn now_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    async fn normalize_path(&self, path: &str) -> Result<String> {
        // 特殊路径不做规范化
        if path == UNGROUPED_WORKSPACE_PATH {
            return Ok(path.to_string());
        }

        let original = path.to_string();
        let normalized = task::spawn_blocking(move || -> Result<String> {
            let candidate = PathBuf::from(&original);
            let canonical = if candidate.exists() {
                std::fs::canonicalize(&candidate)?
            } else {
                candidate
            };

            path_to_string(&canonical)
        })
        .await??;
        Ok(normalized)
    }

    pub async fn get_or_create_workspace(&self, path: &str) -> Result<WorkspaceRecord> {
        let normalized = self.normalize_path(path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "INSERT INTO workspaces (path, display_name, active_session_id, created_at, updated_at, last_accessed_at)
             VALUES (?, NULL, NULL, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at",
        )
        .bind(&normalized)
        .bind(ts)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get_workspace(&normalized)
            .await?
            .ok_or_else(|| anyhow!("Workspace not found after upsert: {}", normalized))
    }

    pub async fn list_recent_workspaces(&self, limit: i64) -> Result<Vec<WorkspaceRecord>> {
        let rows = sqlx::query(
            "SELECT path, display_name, active_session_id, created_at, updated_at, last_accessed_at
             FROM workspaces
             WHERE path != ?
             ORDER BY last_accessed_at DESC LIMIT ?",
        )
        .bind(UNGROUPED_WORKSPACE_PATH)
        .bind(limit.max(1))
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_workspace).collect())
    }

    pub async fn list_sessions(&self, workspace_path: &str) -> Result<Vec<SessionRecord>> {
        let normalized = self.normalize_path(workspace_path).await?;
        let rows = sqlx::query(
            "SELECT s.id, s.workspace_path, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM session_messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s
             WHERE s.workspace_path = ?
             ORDER BY s.updated_at DESC, s.id DESC",
        )
        .bind(&normalized)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_session).collect())
    }

    pub async fn create_session(
        &self,
        workspace_path: &str,
        title: Option<&str>,
    ) -> Result<SessionRecord> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        let ts = Self::now_timestamp();
        let result = sqlx::query(
            "INSERT INTO sessions (workspace_path, title, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&workspace.path)
        .bind(title)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        let id = result.last_insert_rowid();
        self.get_session(id)
            .await?
            .ok_or_else(|| anyhow!("Failed to retrieve session {}", id))
    }

    pub async fn get_active_session(&self, workspace_path: &str) -> Result<Option<SessionRecord>> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        match workspace.active_session_id {
            Some(session_id) => self.get_session(session_id).await,
            None => Ok(None),
        }
    }

    pub async fn ensure_active_session(&self, workspace_path: &str) -> Result<SessionRecord> {
        if let Some(session) = self.get_active_session(workspace_path).await? {
            return Ok(session);
        }

        let created = self.create_session(workspace_path, None).await?;
        self.set_active_session(workspace_path, Some(created.id))
            .await?;
        Ok(created)
    }

    pub async fn trim_session_messages(
        &self,
        workspace_path: &str,
        session_id: i64,
        message_id: i64,
    ) -> Result<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow!("Session {} not found", session_id))?;

        if session.workspace_path != normalized {
            return Err(anyhow!(
                "Session {} does not belong to workspace {}",
                session_id,
                workspace_path
            ));
        }

        self.agent_persistence
            .session_messages()
            .delete_messages_from(session_id, message_id)
            .await
            .map_err(|e| anyhow!("Trim session messages failed: {}", e))?;

        self.refresh_session_metadata(session_id).await?;
        Ok(())
    }

    pub async fn set_active_session(
        &self,
        workspace_path: &str,
        session_id: Option<i64>,
    ) -> Result<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "UPDATE workspaces
             SET active_session_id = ?, updated_at = ?, last_accessed_at = ?
             WHERE path = ?",
        )
        .bind(session_id)
        .bind(ts)
        .bind(ts)
        .bind(&normalized)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn get_session_messages(&self, session_id: i64) -> Result<Vec<SessionMessageRecord>> {
        let rows = sqlx::query(
            "SELECT id, session_id, role, content, steps_json, images_json, status, duration_ms, created_at
             FROM session_messages
             WHERE session_id = ?
             ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_session_message).collect())
    }

    pub async fn delete_workspace(&self, path: &str) -> Result<()> {
        let normalized = self.normalize_path(path).await?;
        sqlx::query("DELETE FROM workspaces WHERE path = ?")
            .bind(&normalized)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn maintain(&self, max_age_days: i64, max_entries: i64) -> Result<(u64, u64)> {
        let cutoff = Self::now_timestamp() - max_age_days * 24 * 60 * 60;

        let deleted_expired = sqlx::query("DELETE FROM workspaces WHERE last_accessed_at < ?")
            .bind(cutoff)
            .execute(self.pool())
            .await?
            .rows_affected();

        let excess = sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.pool())
            .await?
            .unwrap_or(0)
            .saturating_sub(max_entries);

        if excess > 0 {
            sqlx::query(
                "DELETE FROM workspaces WHERE path IN (
                    SELECT path FROM workspaces
                    ORDER BY last_accessed_at DESC
                    LIMIT -1 OFFSET ?
                )",
            )
            .bind(max_entries)
            .execute(self.pool())
            .await?;
        }

        Ok((deleted_expired, excess.max(0) as u64))
    }

    async fn get_workspace(&self, path: &str) -> Result<Option<WorkspaceRecord>> {
        let row = sqlx::query(
            "SELECT path, display_name, active_session_id, created_at, updated_at, last_accessed_at
             FROM workspaces WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(build_workspace))
    }

    pub async fn get_session(&self, id: i64) -> Result<Option<SessionRecord>> {
        let row = sqlx::query(
            "SELECT s.id, s.workspace_path, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM session_messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s WHERE s.id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(build_session))
    }
}

fn path_to_string(path: &Path) -> Result<String> {
    let display = path
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    Ok(display)
}

fn build_workspace(row: sqlx::sqlite::SqliteRow) -> WorkspaceRecord {
    WorkspaceRecord {
        path: row.try_get("path").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or(None),
        active_session_id: row.try_get("active_session_id").unwrap_or(None),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
        last_accessed_at: row.try_get("last_accessed_at").unwrap_or_default(),
    }
}

fn build_session(row: sqlx::sqlite::SqliteRow) -> SessionRecord {
    SessionRecord {
        id: row.try_get("id").unwrap_or_default(),
        workspace_path: row.try_get("workspace_path").unwrap_or_default(),
        title: row.try_get("title").unwrap_or(None),
        message_count: row.try_get("message_count").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn build_session_message(row: sqlx::sqlite::SqliteRow) -> SessionMessageRecord {
    SessionMessageRecord {
        id: row.try_get("id").unwrap_or_default(),
        session_id: row.try_get("session_id").unwrap_or_default(),
        role: row.try_get("role").unwrap_or_default(),
        content: row.try_get("content").unwrap_or(None),
        steps_json: row.try_get("steps_json").unwrap_or(None),
        images_json: row.try_get("images_json").unwrap_or(None),
        status: row.try_get("status").unwrap_or(None),
        duration_ms: row.try_get("duration_ms").unwrap_or(None),
        created_at: row.try_get("created_at").unwrap_or_default(),
    }
}

impl WorkspaceService {
    async fn refresh_session_metadata(&self, session_id: i64) -> Result<()> {
        let latest_user_content: Option<String> = sqlx::query_scalar(
            "SELECT content FROM session_messages
             WHERE session_id = ? AND role = 'user'
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(self.pool())
        .await?
        .flatten();

        let last_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT MAX(created_at) FROM session_messages WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(self.pool())
                .await?;

        let updated_at = last_timestamp.unwrap_or_else(Self::now_timestamp);

        sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(latest_user_content.as_deref())
            .bind(updated_at)
            .bind(session_id)
            .execute(self.pool())
            .await?;

        Ok(())
    }
}
