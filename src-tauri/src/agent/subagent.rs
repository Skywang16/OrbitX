use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::types::{SubagentRecord, SubagentStatus};
use crate::storage::DatabaseManager;

#[derive(Debug, Clone)]
pub struct CreateSubagentParams<'a> {
    pub id: &'a str,
    pub parent_thread_id: i64,
    pub child_thread_id: i64,
    pub parent_message_id: i64,
    pub name: &'a str,
    pub profile: &'a str,
    pub task_title: &'a str,
    pub status: SubagentStatus,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSubagentParams {
    pub status: Option<SubagentStatus>,
    pub latest_activity: Option<Option<String>>,
    pub final_summary: Option<Option<String>>,
    pub error_message: Option<Option<String>>,
    pub finished_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone)]
pub struct SubagentRepository {
    database: Arc<DatabaseManager>,
}

impl SubagentRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn create(&self, params: CreateSubagentParams<'_>) -> AgentResult<SubagentRecord> {
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO subagents (
                id, parent_thread_id, child_thread_id, parent_message_id,
                name, profile, task_title, status,
                latest_activity, final_summary, error_message,
                created_at, updated_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?, NULL)",
        )
        .bind(params.id)
        .bind(params.parent_thread_id)
        .bind(params.child_thread_id)
        .bind(params.parent_message_id)
        .bind(params.name)
        .bind(params.profile)
        .bind(params.task_title)
        .bind(params.status.as_str())
        .bind(now)
        .bind(now)
        .execute(self.pool())
        .await?;

        self.get(params.id).await?.ok_or_else(|| {
            AgentError::Internal(format!(
                "failed to load newly created subagent {}",
                params.id
            ))
        })
    }

    pub async fn get(&self, id: &str) -> AgentResult<Option<SubagentRecord>> {
        let row = sqlx::query("SELECT * FROM subagents WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;

        row.map(|row| build_subagent_record(&row)).transpose()
    }

    pub async fn list_by_parent_thread(
        &self,
        parent_thread_id: i64,
    ) -> AgentResult<Vec<SubagentRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM subagents WHERE parent_thread_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(parent_thread_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_subagent_record(&row))
            .collect()
    }

    pub async fn update(
        &self,
        id: &str,
        params: UpdateSubagentParams,
    ) -> AgentResult<SubagentRecord> {
        let current = self
            .get(id)
            .await?
            .ok_or_else(|| AgentError::Internal(format!("subagent {id} not found")))?;

        let status = params.status.unwrap_or(current.status);
        let latest_activity = match params.latest_activity {
            Some(value) => value,
            None => current.latest_activity,
        };
        let final_summary = match params.final_summary {
            Some(value) => value,
            None => current.final_summary,
        };
        let error_message = match params.error_message {
            Some(value) => value,
            None => current.error_message,
        };
        let finished_at = match params.finished_at {
            Some(value) => value,
            None => current.finished_at,
        };
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE subagents
             SET status = ?, latest_activity = ?, final_summary = ?, error_message = ?, finished_at = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(latest_activity.as_deref())
        .bind(final_summary.as_deref())
        .bind(error_message.as_deref())
        .bind(finished_at.map(|value| value.timestamp()))
        .bind(updated_at)
        .bind(id)
        .execute(self.pool())
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| AgentError::Internal(format!("subagent {id} disappeared after update")))
    }

    pub async fn delete(&self, id: &str) -> AgentResult<()> {
        sqlx::query("DELETE FROM subagents WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_from_parent_message(
        &self,
        parent_thread_id: i64,
        message_id: i64,
    ) -> AgentResult<Vec<SubagentRecord>> {
        let subagents = self.list_by_parent_thread(parent_thread_id).await?;
        let stale = subagents
            .into_iter()
            .filter(|record| record.parent_message_id >= message_id)
            .collect::<Vec<_>>();

        for record in &stale {
            self.delete(&record.id).await?;
        }

        Ok(stale)
    }
}

fn build_subagent_record(row: &sqlx::sqlite::SqliteRow) -> AgentResult<SubagentRecord> {
    let created_at = row.try_get::<i64, _>("created_at")?;
    let updated_at = row.try_get::<i64, _>("updated_at")?;
    let finished_at = row.try_get::<Option<i64>, _>("finished_at")?;

    Ok(SubagentRecord {
        id: row.try_get("id")?,
        parent_thread_id: row.try_get("parent_thread_id")?,
        child_thread_id: row.try_get("child_thread_id")?,
        parent_message_id: row.try_get("parent_message_id")?,
        name: row.try_get("name")?,
        profile: row.try_get("profile")?,
        task_title: row.try_get("task_title")?,
        status: SubagentStatus::from_db(row.try_get::<String, _>("status")?.as_str()),
        latest_activity: row.try_get("latest_activity")?,
        final_summary: row.try_get("final_summary")?,
        error_message: row.try_get("error_message")?,
        created_at: DateTime::<Utc>::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
        updated_at: DateTime::<Utc>::from_timestamp(updated_at, 0).unwrap_or_else(Utc::now),
        finished_at: finished_at.and_then(|value| DateTime::<Utc>::from_timestamp(value, 0)),
    })
}
