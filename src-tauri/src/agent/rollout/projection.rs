use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::agent::error::AgentResult;
use crate::storage::DatabaseManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRecord {
    pub id: i64,
    pub workspace_path: String,
    pub parent_thread_id: Option<i64>,
    pub spawned_by_tool_call_id: Option<String>,
    pub title: String,
    pub display_name: Option<String>,
    pub thread_type: String,
    pub agent_type: String,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub rollout_path: String,
    pub worktree_path: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub total_tokens: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub first_user_message: Option<String>,
}

#[derive(Debug)]
pub struct ThreadRepository {
    database: Arc<DatabaseManager>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateThreadParams<'a> {
    pub workspace_path: &'a str,
    pub title: &'a str,
    pub display_name: Option<&'a str>,
    pub thread_type: &'a str,
    pub agent_type: &'a str,
    pub parent_thread_id: Option<i64>,
    pub spawned_by_tool_call_id: Option<&'a str>,
    pub rollout_path: &'a str,
    pub worktree_path: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub provider_id: Option<&'a str>,
}

impl ThreadRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn get(&self, thread_id: i64) -> AgentResult<Option<ThreadRecord>> {
        let row = sqlx::query("SELECT * FROM threads WHERE id = ?")
            .bind(thread_id)
            .fetch_optional(self.pool())
            .await?;
        row.map(|value| build_thread(&value)).transpose()
    }

    pub async fn create(&self, params: CreateThreadParams<'_>) -> AgentResult<ThreadRecord> {
        let ts = Utc::now().timestamp();
        let result = sqlx::query(
            "INSERT INTO threads (
                workspace_path, parent_thread_id, spawned_by_tool_call_id, title, display_name,
                thread_type, agent_type, model_id, provider_id, rollout_path, worktree_path,
                created_at, updated_at, last_event_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(params.workspace_path)
        .bind(params.parent_thread_id)
        .bind(params.spawned_by_tool_call_id)
        .bind(params.title)
        .bind(params.display_name)
        .bind(params.thread_type)
        .bind(params.agent_type)
        .bind(params.model_id)
        .bind(params.provider_id)
        .bind(params.rollout_path)
        .bind(params.worktree_path)
        .bind(ts)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;
        let id = result.last_insert_rowid();
        self.get(id).await?.ok_or_else(|| {
            crate::agent::error::AgentError::Internal("Failed to create thread".to_string())
        })
    }

    pub async fn update_status(&self, id: i64, status: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads SET status = ?, updated_at = ?, last_event_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_agent_type(&self, id: i64, agent_type: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads SET agent_type = ?, updated_at = ?, last_event_at = ? WHERE id = ?",
        )
        .bind(agent_type)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_display_name(&self, id: i64, display_name: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads SET display_name = ?, updated_at = ?, last_event_at = ? WHERE id = ?",
        )
        .bind(display_name)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_model_id(&self, id: i64, model_id: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads SET model_id = ?, updated_at = ?, last_event_at = ? WHERE id = ?",
        )
        .bind(model_id)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn set_worktree_path(&self, id: i64, worktree_path: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads SET worktree_path = ?, updated_at = ?, last_event_at = ? WHERE id = ?",
        )
        .bind(worktree_path)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_model_selection(
        &self,
        id: i64,
        model_id: &str,
        provider_id: Option<&str>,
    ) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads
             SET model_id = ?, provider_id = ?, updated_at = ?, last_event_at = ?
             WHERE id = ?",
        )
        .bind(model_id)
        .bind(provider_id)
        .bind(ts)
        .bind(ts)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_rollout_path(&self, id: i64, rollout_path: &str) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query("UPDATE threads SET rollout_path = ?, updated_at = ? WHERE id = ?")
            .bind(rollout_path)
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_children(&self, parent_thread_id: i64) -> AgentResult<Vec<ThreadRecord>> {
        let rows = sqlx::query("SELECT * FROM threads WHERE parent_thread_id = ? ORDER BY id ASC")
            .bind(parent_thread_id)
            .fetch_all(self.pool())
            .await?;
        rows.into_iter()
            .map(|row| build_thread(&row))
            .collect::<AgentResult<Vec<_>>>()
    }

    pub async fn delete(&self, id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM threads WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn touch(
        &self,
        thread_id: i64,
        status: Option<&str>,
        title: Option<&str>,
        first_user_message: Option<&str>,
    ) -> AgentResult<()> {
        let ts = Utc::now().timestamp();
        sqlx::query(
            "UPDATE threads
             SET updated_at = ?,
                 last_event_at = ?,
                 status = COALESCE(?, status),
                 title = COALESCE(?, title),
                 first_user_message = COALESCE(?, first_user_message)
             WHERE id = ?",
        )
        .bind(ts)
        .bind(ts)
        .bind(status)
        .bind(title)
        .bind(first_user_message)
        .bind(thread_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

fn build_thread(row: &sqlx::sqlite::SqliteRow) -> AgentResult<ThreadRecord> {
    let created_at = row.try_get::<i64, _>("created_at")?;
    let updated_at = row.try_get::<i64, _>("updated_at")?;
    let last_event_at = row.try_get::<Option<i64>, _>("last_event_at")?;
    Ok(ThreadRecord {
        id: row.try_get("id")?,
        workspace_path: row.try_get("workspace_path")?,
        parent_thread_id: row.try_get("parent_thread_id")?,
        spawned_by_tool_call_id: row.try_get("spawned_by_tool_call_id")?,
        title: row.try_get("title")?,
        display_name: row.try_get("display_name")?,
        thread_type: row.try_get("thread_type")?,
        agent_type: row.try_get("agent_type")?,
        model_id: row.try_get("model_id")?,
        provider_id: row.try_get("provider_id")?,
        rollout_path: row.try_get("rollout_path")?,
        worktree_path: row.try_get("worktree_path")?,
        status: row.try_get("status")?,
        is_archived: row.try_get::<i64, _>("is_archived")? != 0,
        total_tokens: row.try_get("total_tokens")?,
        created_at: chrono::DateTime::<Utc>::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
        updated_at: chrono::DateTime::<Utc>::from_timestamp(updated_at, 0).unwrap_or_else(Utc::now),
        last_event_at: last_event_at
            .and_then(|value| chrono::DateTime::<Utc>::from_timestamp(value, 0)),
        first_user_message: row.try_get("first_user_message")?,
    })
}
