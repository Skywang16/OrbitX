use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{self, sqlite::SqliteQueryResult, Row};

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::types::{
    Block, Message, MessageRole as UiMessageRole, MessageStatus as UiMessageStatus, TokenUsage,
};
use crate::storage::database::DatabaseManager;

use super::models::{
    build_agent_execution, build_execution_event, build_execution_message, build_session,
    build_session_summary, build_tool_execution, build_workspace,
    build_workspace_file_record, AgentExecution, ExecutionEvent, ExecutionEventType,
    ExecutionMessage, ExecutionStatus, FileRecordSource, FileRecordState,
    MessageRole as AgentMessageRole,
    Session, SessionSummary, TokenUsageStats, ToolExecution, ToolExecutionStatus, Workspace, WorkspaceFileRecord,
};
use super::{
    bool_to_sql, now_timestamp, opt_datetime_to_timestamp, opt_timestamp_to_datetime,
    timestamp_to_datetime,
};

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

    pub async fn upsert(&self, path: &str, display_name: Option<&str>) -> AgentResult<Workspace> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO workspaces (path, display_name, active_session_id, created_at, updated_at, last_accessed_at)
             VALUES (?, ?, NULL, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                display_name = COALESCE(excluded.display_name, workspaces.display_name),
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at",
        )
        .bind(path)
        .bind(display_name)
        .bind(ts)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get(path)
            .await?
            .ok_or_else(|| AgentError::Internal(format!("Failed to upsert workspace {}", path)))
    }

    pub async fn get(&self, path: &str) -> AgentResult<Option<Workspace>> {
        let row =
            sqlx::query("SELECT path, display_name, active_session_id, created_at, updated_at, last_accessed_at FROM workspaces WHERE path = ?")
                .bind(path)
                .fetch_optional(self.pool())
                .await?;
        Ok(row.map(|r| build_workspace(&r)))
    }

    pub async fn set_active_session(&self, path: &str, session_id: Option<i64>) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE workspaces
             SET active_session_id = ?, updated_at = ?, last_accessed_at = ?
             WHERE path = ?",
        )
        .bind(session_id)
        .bind(ts)
        .bind(ts)
        .bind(path)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn touch(&self, path: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE workspaces SET updated_at = ?, last_accessed_at = ? WHERE path = ?")
            .bind(ts)
            .bind(ts)
            .bind(path)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_recent(&self, limit: i64) -> AgentResult<Vec<Workspace>> {
        let rows = sqlx::query(
            "SELECT path, display_name, active_session_id, created_at, updated_at, last_accessed_at
             FROM workspaces ORDER BY last_accessed_at DESC LIMIT ?",
        )
        .bind(limit.max(1))
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(|r| build_workspace(&r)).collect())
    }

    pub async fn delete(&self, path: &str) -> AgentResult<()> {
        sqlx::query("DELETE FROM workspaces WHERE path = ?")
            .bind(path)
            .execute(self.pool())
            .await?;
        Ok(())
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

    pub async fn create(&self, workspace_path: &str, title: Option<&str>) -> AgentResult<Session> {
        let ts = now_timestamp();
        let result: SqliteQueryResult = sqlx::query(
            "INSERT INTO sessions (workspace_path, title, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(workspace_path)
        .bind(title)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get(result.last_insert_rowid())
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to create session".to_string()))
    }

    pub async fn get(&self, id: i64) -> AgentResult<Option<Session>> {
        let row = sqlx::query("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        Ok(row.map(|r| build_session(&r)))
    }

    pub async fn list_by_workspace(&self, workspace_path: &str) -> AgentResult<Vec<Session>> {
        let rows = sqlx::query(
            "SELECT * FROM sessions
             WHERE workspace_path = ?
             ORDER BY updated_at DESC, id DESC",
        )
        .bind(workspace_path)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(|r| build_session(&r)).collect())
    }

    pub async fn touch(&self, id: i64) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn update_title(&self, id: i64, title: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SessionSummaryRepository {
    database: Arc<DatabaseManager>,
}

impl SessionSummaryRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn upsert(
        &self,
        session_id: i64,
        summary_content: &str,
        summary_tokens: i64,
        messages_summarized: i64,
        tokens_saved: i64,
    ) -> AgentResult<SessionSummary> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO session_summaries (
                session_id, summary_content, summary_tokens,
                messages_summarized, tokens_saved, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_id) DO UPDATE SET
                summary_content = excluded.summary_content,
                summary_tokens = excluded.summary_tokens,
                messages_summarized = excluded.messages_summarized,
                tokens_saved = excluded.tokens_saved,
                updated_at = excluded.updated_at",
        )
        .bind(session_id)
        .bind(summary_content)
        .bind(summary_tokens)
        .bind(messages_summarized)
        .bind(tokens_saved)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get(session_id).await?.ok_or_else(|| {
            AgentError::Internal(format!("Failed to retrieve summary {}", session_id))
        })
    }

    pub async fn get(&self, session_id: i64) -> AgentResult<Option<SessionSummary>> {
        let row = sqlx::query("SELECT * FROM session_summaries WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(self.pool())
            .await?;

        Ok(row.map(|r| build_session_summary(&r)))
    }

    pub async fn delete(&self, session_id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM session_summaries WHERE session_id = ?")
            .bind(session_id)
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
            "SELECT id, session_id, role, status, blocks_json, created_at, finished_at, duration_ms,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens
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

    pub async fn get(&self, message_id: i64) -> AgentResult<Option<Message>> {
        let row = sqlx::query(
            "SELECT id, session_id, role, status, blocks_json, created_at, finished_at, duration_ms,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens
             FROM messages
             WHERE id = ?",
        )
        .bind(message_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_message(&r)).transpose()
    }

    pub async fn create(
        &self,
        session_id: i64,
        role: UiMessageRole,
        status: UiMessageStatus,
        blocks: Vec<Block>,
    ) -> AgentResult<Message> {
        let ts = now_timestamp();
        let blocks_json = serde_json::to_string(&blocks).map_err(|e| {
            AgentError::Internal(format!("Failed to serialize message blocks: {}", e))
        })?;

        let result = sqlx::query(
            "INSERT INTO messages (session_id, role, status, blocks_json, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(role_as_str(&role))
        .bind(status_as_str(&status))
        .bind(blocks_json)
        .bind(ts)
        .execute(self.pool())
        .await?;

        let message_id = result.last_insert_rowid();

        // Refresh session metadata for list ordering + default title.
        self.touch_session_on_message_create(session_id, ts, &role, &blocks)
            .await?;

        Ok(Message {
            id: message_id,
            session_id,
            role,
            status,
            blocks,
            created_at: timestamp_to_datetime(ts),
            finished_at: None,
            duration_ms: None,
            token_usage: None,
        })
    }

    pub async fn update(&self, message: &Message) -> AgentResult<()> {
        let blocks_json = serde_json::to_string(&message.blocks).map_err(|e| {
            AgentError::Internal(format!("Failed to serialize message blocks: {}", e))
        })?;

        let (input_tokens, output_tokens, cache_read_tokens, cache_write_tokens) =
            token_usage_to_columns(message.token_usage.as_ref());

        sqlx::query(
            "UPDATE messages
             SET status = ?,
                 blocks_json = ?,
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

    /// 删除指定消息及其之后的所有消息
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

    async fn touch_session_on_message_create(
        &self,
        session_id: i64,
        created_at: i64,
        role: &UiMessageRole,
        blocks: &[Block],
    ) -> AgentResult<()> {
        // Always bump updated_at for ordering.
        sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
            .bind(created_at)
            .bind(session_id)
            .execute(self.pool())
            .await?;

        if !matches!(role, UiMessageRole::User) {
            return Ok(());
        }

        let Some(content) = blocks.iter().find_map(|b| match b {
            Block::UserText(t) => Some(t.content.as_str()),
            _ => None,
        }) else {
            return Ok(());
        };

        let Some(title) = derive_session_title(content) else {
            return Ok(());
        };

        sqlx::query(
            "UPDATE sessions
             SET title = ?, updated_at = ?
             WHERE id = ? AND (title IS NULL OR trim(title) = '')",
        )
        .bind(&title)
        .bind(created_at)
        .bind(session_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}

const SESSION_TITLE_MAX_LENGTH: usize = 30;

fn derive_session_title(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = String::with_capacity(trimmed.len());
    let mut last_was_space = false;

    for ch in trimmed.chars() {
        let mapped = match ch {
            '\r' | '\n' => ' ',
            other => other,
        };
        if mapped.is_whitespace() {
            if last_was_space {
                continue;
            }
            normalized.push(' ');
            last_was_space = true;
        } else {
            normalized.push(mapped);
            last_was_space = false;
        }
    }

    let normalized = normalized.trim();
    if normalized.is_empty() {
        return None;
    }

    let mut title = String::new();
    let mut count = 0;
    let mut truncated = false;

    for ch in normalized.chars() {
        if count >= SESSION_TITLE_MAX_LENGTH {
            truncated = true;
            break;
        }
        title.push(ch);
        count += 1;
    }

    if truncated {
        title.push_str("...");
    }

    Some(title)
}

fn role_as_str(role: &UiMessageRole) -> &'static str {
    match role {
        UiMessageRole::User => "user",
        UiMessageRole::Assistant => "assistant",
    }
}

fn status_as_str(status: &UiMessageStatus) -> &'static str {
    match status {
        UiMessageStatus::Streaming => "streaming",
        UiMessageStatus::Completed => "completed",
        UiMessageStatus::Cancelled => "cancelled",
        UiMessageStatus::Error => "error",
    }
}

fn parse_role(role: &str) -> AgentResult<UiMessageRole> {
    match role {
        "user" => Ok(UiMessageRole::User),
        "assistant" => Ok(UiMessageRole::Assistant),
        other => Err(AgentError::Parse(format!("Unknown message role: {}", other))),
    }
}

fn parse_status(status: &str) -> AgentResult<UiMessageStatus> {
    match status {
        "streaming" => Ok(UiMessageStatus::Streaming),
        "completed" => Ok(UiMessageStatus::Completed),
        "cancelled" => Ok(UiMessageStatus::Cancelled),
        "error" => Ok(UiMessageStatus::Error),
        other => Err(AgentError::Parse(format!("Unknown message status: {}", other))),
    }
}

fn token_usage_to_columns(
    usage: Option<&TokenUsage>,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let Some(usage) = usage else {
        return (None, None, None, None);
    };
    (
        Some(usage.input_tokens),
        Some(usage.output_tokens),
        usage.cache_read_tokens,
        usage.cache_write_tokens,
    )
}

fn build_message(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Message> {
    let blocks_json: String = row.try_get("blocks_json")?;
    let blocks: Vec<Block> = serde_json::from_str(&blocks_json).map_err(|e| {
        AgentError::Parse(format!("Invalid message blocks_json: {}", e))
    })?;

    let role = parse_role(row.try_get::<String, _>("role")?.as_str())?;
    let status = parse_status(row.try_get::<String, _>("status")?.as_str())?;

    let token_usage = match (
        row.try_get::<Option<i64>, _>("input_tokens")?,
        row.try_get::<Option<i64>, _>("output_tokens")?,
        row.try_get::<Option<i64>, _>("cache_read_tokens")?,
        row.try_get::<Option<i64>, _>("cache_write_tokens")?,
    ) {
        (Some(input_tokens), Some(output_tokens), cache_read_tokens, cache_write_tokens) => {
            Some(TokenUsage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
            })
        }
        _ => None,
    };

    Ok(Message {
        id: row.try_get("id")?,
        session_id: row.try_get("session_id")?,
        role,
        status,
        blocks,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        finished_at: opt_timestamp_to_datetime(row.try_get("finished_at")?),
        duration_ms: row.try_get("duration_ms")?,
        token_usage,
    })
}

#[derive(Debug)]
pub struct WorkspaceFileContextRepository {
    database: Arc<DatabaseManager>,
}

impl WorkspaceFileContextRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_entry(
        &self,
        workspace_path: &str,
        relative_path: &str,
        record_state: FileRecordState,
        record_source: FileRecordSource,
        agent_read_at: Option<DateTime<Utc>>,
        agent_edit_at: Option<DateTime<Utc>>,
        user_edit_at: Option<DateTime<Utc>>,
    ) -> AgentResult<WorkspaceFileRecord> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO workspace_file_context (
                workspace_path, relative_path, record_state, record_source,
                agent_read_at, agent_edit_at, user_edit_at, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(workspace_path, relative_path) DO UPDATE SET
                record_state = excluded.record_state,
                record_source = excluded.record_source,
                agent_read_at = excluded.agent_read_at,
                agent_edit_at = excluded.agent_edit_at,
                user_edit_at = excluded.user_edit_at",
        )
        .bind(workspace_path)
        .bind(relative_path)
        .bind(record_state.as_str())
        .bind(record_source.as_str())
        .bind(opt_datetime_to_timestamp(agent_read_at))
        .bind(opt_datetime_to_timestamp(agent_edit_at))
        .bind(opt_datetime_to_timestamp(user_edit_at))
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.find_by_path(workspace_path, relative_path)
            .await?
            .ok_or_else(|| {
                AgentError::Internal(format!("Failed to fetch file context {}", relative_path))
            })
    }

    pub async fn find_by_path(
        &self,
        workspace_path: &str,
        relative_path: &str,
    ) -> AgentResult<Option<WorkspaceFileRecord>> {
        let row = sqlx::query(
            "SELECT * FROM workspace_file_context
             WHERE workspace_path = ? AND relative_path = ?",
        )
        .bind(workspace_path)
        .bind(relative_path)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_workspace_file_record(&r)).transpose()
    }

    pub async fn list_by_state(
        &self,
        workspace_path: &str,
        state: FileRecordState,
    ) -> AgentResult<Vec<WorkspaceFileRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM workspace_file_context
             WHERE workspace_path = ? AND record_state = ?
             ORDER BY created_at DESC",
        )
        .bind(workspace_path)
        .bind(state.as_str())
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|r| build_workspace_file_record(&r))
            .collect()
    }

    pub async fn list_agent_edited_files(
        &self,
        workspace_path: &str,
    ) -> AgentResult<Vec<WorkspaceFileRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM workspace_file_context
             WHERE workspace_path = ? AND agent_edit_at IS NOT NULL
             ORDER BY agent_edit_at DESC",
        )
        .bind(workspace_path)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|r| build_workspace_file_record(&r))
            .collect()
    }
}

#[derive(Debug)]
pub struct AgentExecutionRepository {
    database: Arc<DatabaseManager>,
}

impl AgentExecutionRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        execution_id: &str,
        session_id: i64,
        user_request: &str,
        system_prompt_used: &str,
        execution_config: Option<&str>,
        has_context: bool,
        max_iterations: i64,
    ) -> AgentResult<AgentExecution> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO agent_executions (
                execution_id, session_id, user_request, system_prompt_used, execution_config,
                has_conversation_context, status, current_iteration, error_count, max_iterations,
                total_input_tokens, total_output_tokens, total_cost, context_tokens,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, 'running', 0, 0, ?, 0, 0, 0.0, 0, ?, ?)",
        )
        .bind(execution_id)
        .bind(session_id)
        .bind(user_request)
        .bind(system_prompt_used)
        .bind(execution_config)
        .bind(bool_to_sql(has_context))
        .bind(max_iterations)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get_by_execution_id(execution_id)
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to create execution".to_string()))
    }

    pub async fn get_by_execution_id(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<AgentExecution>> {
        let row = sqlx::query("SELECT * FROM agent_executions WHERE execution_id = ?")
            .bind(execution_id)
            .fetch_optional(self.pool())
            .await?;

        row.map(|r| build_agent_execution(&r)).transpose()
    }

    pub async fn list_recent_by_session(
        &self,
        session_id: i64,
        limit: i64,
    ) -> AgentResult<Vec<AgentExecution>> {
        let rows = sqlx::query(
            "SELECT * FROM agent_executions
             WHERE session_id = ?
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|r| build_agent_execution(&r))
            .collect()
    }

    pub async fn list_recent(&self, limit: i64) -> AgentResult<Vec<AgentExecution>> {
        let rows = sqlx::query("SELECT * FROM agent_executions ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(self.pool())
            .await?;

        rows.into_iter()
            .map(|r| build_agent_execution(&r))
            .collect()
    }

    pub async fn mark_started(&self, execution_id: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET started_at = ?, status = 'running', updated_at = ?
             WHERE execution_id = ?",
        )
        .bind(ts)
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_status(
        &self,
        execution_id: &str,
        status: ExecutionStatus,
        current_iteration: i64,
        error_count: i64,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET status = ?, current_iteration = ?, error_count = ?, updated_at = ?
             WHERE execution_id = ?",
        )
        .bind(status.as_str())
        .bind(current_iteration)
        .bind(error_count)
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn set_has_context(&self, execution_id: &str, has_context: bool) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET has_conversation_context = ?, updated_at = ?
             WHERE execution_id = ?",
        )
        .bind(bool_to_sql(has_context))
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_execution_config(
        &self,
        execution_id: &str,
        config: Option<&str>,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET execution_config = ?, updated_at = ? WHERE execution_id = ?",
        )
        .bind(config)
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn mark_finished(
        &self,
        execution_id: &str,
        status: ExecutionStatus,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET status = ?, completed_at = ?, updated_at = ?
             WHERE execution_id = ?",
        )
        .bind(status.as_str())
        .bind(ts)
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_token_usage(
        &self,
        execution_id: &str,
        total_input_tokens: i64,
        total_output_tokens: i64,
        context_tokens: i64,
        total_cost: f64,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions
             SET total_input_tokens = ?,
                 total_output_tokens = ?,
                 context_tokens = ?,
                 total_cost = ?,
                 updated_at = ?
             WHERE execution_id = ?",
        )
        .bind(total_input_tokens)
        .bind(total_output_tokens)
        .bind(context_tokens)
        .bind(total_cost)
        .bind(ts)
        .bind(execution_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn aggregate_token_usage(&self, session_id: i64) -> AgentResult<TokenUsageStats> {
        let row = sqlx::query(
            "SELECT
                COALESCE(SUM(total_input_tokens), 0) AS total_input_tokens,
                COALESCE(SUM(total_output_tokens), 0) AS total_output_tokens,
                COALESCE(SUM(context_tokens), 0) AS total_context_tokens,
                COALESCE(SUM(total_cost), 0.0) AS total_cost
             FROM agent_executions WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(self.pool())
        .await?;

        Ok(TokenUsageStats::new(
            row.try_get("total_input_tokens")?,
            row.try_get("total_output_tokens")?,
            row.try_get("total_context_tokens")?,
            row.try_get("total_cost")?,
        ))
    }

    pub async fn delete_by_session(&self, session_id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM agent_executions WHERE session_id = ?")
            .bind(session_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_after(&self, session_id: i64, cutoff_timestamp: i64) -> AgentResult<()> {
        sqlx::query(
            "DELETE FROM agent_executions
             WHERE session_id = ? AND created_at >= ?",
        )
        .bind(session_id)
        .bind(cutoff_timestamp)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ExecutionMessageRepository {
    database: Arc<DatabaseManager>,
}

impl ExecutionMessageRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn append_message(
        &self,
        execution_id: &str,
        role: AgentMessageRole,
        content: &str,
        tokens: i64,
        is_summary: bool,
        iteration: i64,
        sequence: i64,
    ) -> AgentResult<ExecutionMessage> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO execution_messages (
                execution_id, role, content, tokens, is_summary,
                iteration, sequence, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(execution_id)
        .bind(role.as_str())
        .bind(content)
        .bind(tokens)
        .bind(bool_to_sql(is_summary))
        .bind(iteration)
        .bind(sequence)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.latest_for_execution(execution_id)
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to append execution message".to_string()))
    }

    pub async fn latest_for_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<ExecutionMessage>> {
        let row = sqlx::query(
            "SELECT * FROM execution_messages
             WHERE execution_id = ?
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_execution_message(&r)).transpose()
    }

    pub async fn list_by_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Vec<ExecutionMessage>> {
        let rows = sqlx::query(
            "SELECT * FROM execution_messages
             WHERE execution_id = ?
             ORDER BY iteration ASC, sequence ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|r| build_execution_message(&r))
            .collect()
    }

    pub async fn delete_for_execution(&self, execution_id: &str) -> AgentResult<()> {
        sqlx::query("DELETE FROM execution_messages WHERE execution_id = ?")
            .bind(execution_id)
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

    #[allow(clippy::too_many_arguments)]
    pub async fn record_execution(
        &self,
        execution_id: &str,
        tool_call_id: &str,
        tool_name: &str,
        tool_arguments: &str,
        status: ToolExecutionStatus,
        files_read: &str,
        files_written: &str,
        directories_accessed: &str,
    ) -> AgentResult<ToolExecution> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO tool_executions (
                execution_id, tool_call_id, tool_name, tool_arguments, status,
                files_read, files_written, directories_accessed, started_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(execution_id)
        .bind(tool_call_id)
        .bind(tool_name)
        .bind(tool_arguments)
        .bind(status.as_str())
        .bind(files_read)
        .bind(files_written)
        .bind(directories_accessed)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.last_for_execution(execution_id)
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to record tool execution".to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_status(
        &self,
        tool_call_id: &str,
        status: ToolExecutionStatus,
        tool_result: Option<&str>,
        error_message: Option<&str>,
        completed_at: Option<DateTime<Utc>>,
        duration_ms: Option<i64>,
    ) -> AgentResult<()> {
        sqlx::query(
            "UPDATE tool_executions
             SET status = ?,
                 tool_result = COALESCE(?, tool_result),
                 error_message = COALESCE(?, error_message),
                 completed_at = COALESCE(?, completed_at),
                 duration_ms = COALESCE(?, duration_ms)
             WHERE tool_call_id = ?",
        )
        .bind(status.as_str())
        .bind(tool_result)
        .bind(error_message)
        .bind(opt_datetime_to_timestamp(completed_at))
        .bind(duration_ms)
        .bind(tool_call_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn last_for_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<ToolExecution>> {
        let row = sqlx::query(
            "SELECT * FROM tool_executions
             WHERE execution_id = ?
             ORDER BY started_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_tool_execution(&r)).transpose()
    }

    pub async fn list_by_execution(&self, execution_id: &str) -> AgentResult<Vec<ToolExecution>> {
        let rows = sqlx::query(
            "SELECT * FROM tool_executions
             WHERE execution_id = ?
             ORDER BY started_at ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(|r| build_tool_execution(&r)).collect()
    }
}

#[derive(Debug)]
pub struct ExecutionEventRepository {
    database: Arc<DatabaseManager>,
}

impl ExecutionEventRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn record_event(
        &self,
        execution_id: &str,
        event_type: ExecutionEventType,
        event_data: &str,
        iteration: i64,
    ) -> AgentResult<ExecutionEvent> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO execution_events (
                execution_id, event_type, event_data, iteration, created_at
             ) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(execution_id)
        .bind(event_type.as_str())
        .bind(event_data)
        .bind(iteration)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.last_for_execution(execution_id)
            .await?
            .ok_or_else(|| AgentError::Internal("Failed to record execution event".to_string()))
    }

    pub async fn list_by_execution(&self, execution_id: &str) -> AgentResult<Vec<ExecutionEvent>> {
        let rows = sqlx::query(
            "SELECT * FROM execution_events
             WHERE execution_id = ?
             ORDER BY iteration ASC, created_at ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|r| build_execution_event(&r))
            .collect()
    }

    pub async fn last_for_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<ExecutionEvent>> {
        let row = sqlx::query(
            "SELECT * FROM execution_events
             WHERE execution_id = ?
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_execution_event(&r)).transpose()
    }
}
