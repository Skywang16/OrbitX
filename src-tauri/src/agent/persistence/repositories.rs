use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{self, sqlite::SqliteQueryResult, Row};

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::types::{Block, Message, MessageRole, MessageStatus, TokenUsage};
use crate::storage::database::DatabaseManager;

use super::models::{build_session, build_tool_execution, build_workspace, Session, ToolExecution, Workspace};
use super::{bool_to_sql, now_timestamp, opt_datetime_to_timestamp, opt_timestamp_to_datetime, timestamp_to_datetime};

#[derive(Debug)]
pub struct WorkspaceRepository {
    database: Arc<DatabaseManager>,
}

impl WorkspaceRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn get(&self, path: &str) -> AgentResult<Option<Workspace>> {
        let row = sqlx::query(
            "SELECT path, display_name, active_session_id, created_at, updated_at, last_accessed_at
             FROM workspaces WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(|r| build_workspace(&r)))
    }
}

#[derive(Debug)]
pub struct SessionRepository {
    database: Arc<DatabaseManager>,
}

impl SessionRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn get(&self, id: i64) -> AgentResult<Option<Session>> {
        let row = sqlx::query("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        row.map(|r| build_session(&r)).transpose()
    }

    pub async fn create(
        &self,
        workspace_path: &str,
        title: Option<&str>,
        agent_type: &str,
        parent_id: Option<i64>,
        spawned_by_tool_call: Option<&str>,
        model_id: Option<&str>,
        provider_id: Option<&str>,
    ) -> AgentResult<Session> {
        let ts = now_timestamp();
        let result: SqliteQueryResult = sqlx::query(
            "INSERT INTO sessions (
                workspace_path, parent_id, agent_type, spawned_by_tool_call,
                title, model_id, provider_id,
                status, is_archived,
                total_messages, total_tokens, total_cost,
                created_at, updated_at, last_message_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, 'idle', 0, 0, 0, 0, ?, ?, NULL)",
        )
        .bind(workspace_path)
        .bind(parent_id)
        .bind(agent_type)
        .bind(spawned_by_tool_call)
        .bind(title)
        .bind(model_id)
        .bind(provider_id)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get(result.last_insert_rowid())
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to create session".to_string()))
    }

    pub async fn touch_on_message(&self, session_id: i64, message_ts: i64) -> AgentResult<()> {
        sqlx::query(
            "UPDATE sessions
             SET updated_at = ?, last_message_at = ?,
                 total_messages = total_messages + 1
             WHERE id = ?",
        )
        .bind(message_ts)
        .bind(message_ts)
        .bind(session_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: i64, status: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE sessions SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MessageRepository {
    database: Arc<DatabaseManager>,
}

impl MessageRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn list_by_session(&self, session_id: i64) -> AgentResult<Vec<Message>> {
        let rows = sqlx::query(
            "SELECT
                id, session_id, role, agent_type, parent_message_id,
                status, blocks, is_summary,
                model_id, provider_id,
                input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                created_at, finished_at, duration_ms
             FROM messages
             WHERE session_id = ?
             ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_message(&row))
            .collect::<AgentResult<Vec<_>>>()
    }

    pub async fn create(
        &self,
        session_id: i64,
        role: MessageRole,
        status: MessageStatus,
        blocks: Vec<Block>,
        is_summary: bool,
        agent_type: &str,
        parent_message_id: Option<i64>,
        model_id: Option<&str>,
        provider_id: Option<&str>,
    ) -> AgentResult<Message> {
        let ts = now_timestamp();
        let blocks_json = serde_json::to_string(&blocks).map_err(|e| {
            AgentError::Internal(format!("Failed to serialize message blocks: {e}"))
        })?;

        let result = sqlx::query(
            "INSERT INTO messages (
                session_id, role, agent_type, parent_message_id,
                status, blocks, is_summary,
                model_id, provider_id,
                created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(role_as_str(&role))
        .bind(agent_type)
        .bind(parent_message_id)
        .bind(status_as_str(&status))
        .bind(blocks_json)
        .bind(bool_to_sql(is_summary))
        .bind(model_id)
        .bind(provider_id)
        .bind(ts)
        .execute(self.pool())
        .await?;

        let message_id = result.last_insert_rowid();

        let sessions = SessionRepository::new(Arc::clone(&self.database));
        sessions.touch_on_message(session_id, ts).await?;

        Ok(Message {
            id: message_id,
            session_id,
            role,
            agent_type: agent_type.to_string(),
            parent_message_id,
            status,
            blocks,
            is_summary,
            model_id: model_id.map(|s| s.to_string()),
            provider_id: provider_id.map(|s| s.to_string()),
            created_at: timestamp_to_datetime(ts),
            finished_at: None,
            duration_ms: None,
            token_usage: None,
            context_usage: None,
        })
    }

    pub async fn update(&self, message: &Message) -> AgentResult<()> {
        let blocks_json = serde_json::to_string(&message.blocks).map_err(|e| {
            AgentError::Internal(format!("Failed to serialize message blocks: {e}"))
        })?;

        let (input_tokens, output_tokens, cache_read_tokens, cache_write_tokens) =
            token_usage_to_columns(message.token_usage.as_ref());

        sqlx::query(
            "UPDATE messages
             SET status = ?,
                 blocks = ?,
                 is_summary = ?,
                 finished_at = ?,
                 duration_ms = ?,
                 input_tokens = ?,
                 output_tokens = ?,
                 cache_read_tokens = ?,
                 cache_write_tokens = ?
             WHERE id = ?",
        )
        .bind(status_as_str(&message.status))
        .bind(blocks_json)
        .bind(bool_to_sql(message.is_summary))
        .bind(opt_datetime_to_timestamp(message.finished_at))
        .bind(message.duration_ms)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_write_tokens)
        .bind(message.id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn delete_messages_from(&self, session_id: i64, message_id: i64) -> AgentResult<()> {
        let created_at: i64 = sqlx::query_scalar("SELECT created_at FROM messages WHERE id = ?")
            .bind(message_id)
            .fetch_one(self.pool())
            .await?;

        sqlx::query(
            "DELETE FROM messages
             WHERE session_id = ?
               AND (created_at > ? OR (created_at = ? AND id >= ?))",
        )
        .bind(session_id)
        .bind(created_at)
        .bind(created_at)
        .bind(message_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ToolExecutionRepository {
    database: Arc<DatabaseManager>,
}

impl ToolExecutionRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn create(
        &self,
        message_id: i64,
        session_id: i64,
        call_id: &str,
        tool_name: &str,
        status: &str,
        started_at: DateTime<Utc>,
    ) -> AgentResult<ToolExecution> {
        let started_at_ts = started_at.timestamp();
        let result = sqlx::query(
            "INSERT INTO tool_executions (
                message_id, session_id, call_id, tool_name, status, started_at
             ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(message_id)
        .bind(session_id)
        .bind(call_id)
        .bind(tool_name)
        .bind(status)
        .bind(started_at_ts)
        .execute(self.pool())
        .await?;

        self.get(result.last_insert_rowid())
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to create tool execution".to_string()))
    }

    pub async fn get(&self, id: i64) -> AgentResult<Option<ToolExecution>> {
        let row = sqlx::query("SELECT * FROM tool_executions WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        row.map(|r| build_tool_execution(&r)).transpose()
    }
}

fn role_as_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

fn status_as_str(status: &MessageStatus) -> &'static str {
    match status {
        MessageStatus::Streaming => "streaming",
        MessageStatus::Completed => "completed",
        MessageStatus::Cancelled => "cancelled",
        MessageStatus::Error => "error",
    }
}

fn build_message(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Message> {
    let blocks_json: String = row.try_get("blocks")?;
    let blocks: Vec<Block> = serde_json::from_str(&blocks_json)
        .map_err(|e| AgentError::Parse(format!("Invalid message blocks JSON: {e}")))?;

    let status_raw: String = row.try_get("status")?;
    let status = match status_raw.as_str() {
        "streaming" => MessageStatus::Streaming,
        "completed" => MessageStatus::Completed,
        "cancelled" => MessageStatus::Cancelled,
        "error" => MessageStatus::Error,
        other => return Err(AgentError::Parse(format!("Unknown message status: {other}"))),
    };

    let role_raw: String = row.try_get("role")?;
    let role = match role_raw.as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        other => return Err(AgentError::Parse(format!("Unknown message role: {other}"))),
    };

    let input_tokens: Option<i64> = row.try_get("input_tokens")?;
    let output_tokens: Option<i64> = row.try_get("output_tokens")?;
    let cache_read_tokens: Option<i64> = row.try_get("cache_read_tokens")?;
    let cache_write_tokens: Option<i64> = row.try_get("cache_write_tokens")?;
    let token_usage = if let (Some(input_tokens), Some(output_tokens)) = (input_tokens, output_tokens)
    {
        Some(TokenUsage {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
        })
    } else {
        None
    };

    Ok(Message {
        id: row.try_get("id")?,
        session_id: row.try_get("session_id")?,
        role,
        agent_type: row.try_get("agent_type")?,
        parent_message_id: row.try_get("parent_message_id")?,
        status,
        blocks,
        is_summary: row.try_get::<i64, _>("is_summary")? != 0,
        model_id: row.try_get("model_id")?,
        provider_id: row.try_get("provider_id")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        finished_at: opt_timestamp_to_datetime(row.try_get("finished_at")?),
        duration_ms: row.try_get("duration_ms")?,
        token_usage,
        context_usage: None,
    })
}

fn token_usage_to_columns(
    token_usage: Option<&TokenUsage>,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let Some(token_usage) = token_usage else {
        return (None, None, None, None);
    };
    (
        Some(token_usage.input_tokens),
        Some(token_usage.output_tokens),
        token_usage.cache_read_tokens,
        token_usage.cache_write_tokens,
    )
}
