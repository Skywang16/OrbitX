use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{self, sqlite::SqliteQueryResult, Row};

use crate::agent::error::{AgentError, AgentResult};
use crate::storage::database::DatabaseManager;

use super::models::{
    build_agent_execution, build_conversation, build_conversation_summary, build_execution_event,
    build_execution_message, build_file_context_entry, build_tool_execution, AgentExecution,
    Conversation, ConversationSummary, ExecutionEvent, ExecutionEventType, ExecutionMessage,
    ExecutionStatus, FileContextEntry, FileRecordSource, FileRecordState, MessageRole,
    TokenUsageStats, ToolExecution, ToolExecutionStatus,
};
use super::{bool_to_sql, datetime_to_timestamp, now_timestamp, opt_datetime_to_timestamp};

/// Repository that manages top-level agent conversations.
#[derive(Debug)]
pub struct ConversationRepository {
    database: Arc<DatabaseManager>,
}

impl ConversationRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn create(
        &self,
        title: Option<&str>,
        workspace_path: Option<&str>,
    ) -> AgentResult<Conversation> {
        let ts = now_timestamp();
        let result: SqliteQueryResult = sqlx::query(
            "INSERT INTO conversations (title, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(title)
        .bind(workspace_path)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        let id = result.last_insert_rowid();
        self.get(id).await?.ok_or_else(|| {
            AgentError::Internal(format!("Failed to retrieve created conversation: {}", id))
        })
    }

    /// Ensure a conversation record with the specified id exists. If it does not,
    /// insert a placeholder row so foreign-key references remain valid when the
    /// frontend sends legacy conversation ids from the chat subsystem.
    pub async fn ensure_with_id(
        &self,
        id: i64,
        title: Option<&str>,
        workspace_path: Option<&str>,
    ) -> AgentResult<()> {
        if id <= 0 {
            return Err(AgentError::Internal(format!(
                "Conversation ID must be positive: {}",
                id
            )));
        }

        if self.get(id).await?.is_some() {
            return Ok(());
        }

        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO conversations (id, title, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(id)
        .bind(title)
        .bind(workspace_path)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn get(&self, id: i64) -> AgentResult<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, title, workspace_path, created_at, updated_at
             FROM conversations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_conversation(&row)).transpose()
    }

    /// 检查conversation是否存在
    pub async fn exists(&self, id: i64) -> AgentResult<bool> {
        let row = sqlx::query("SELECT 1 FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        Ok(row.is_some())
    }

    /// 创建具有指定ID的conversation（简单包装ensure_with_id）
    pub async fn create_with_id(
        &self,
        id: i64,
        title: Option<&str>,
        workspace_path: Option<&str>,
        _timestamp: i64,
    ) -> AgentResult<()> {
        self.ensure_with_id(id, title, workspace_path).await
    }

    pub async fn list_recent(&self, limit: i64) -> AgentResult<Vec<Conversation>> {
        let rows = sqlx::query(
            "SELECT id, title, workspace_path, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_conversation(&row))
            .collect()
    }

    pub async fn list_paginated(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AgentResult<Vec<Conversation>> {
        let limit = limit.unwrap_or(50).max(1);
        let offset = offset.unwrap_or(0).max(0);
        let rows = sqlx::query(
            "SELECT id, title, workspace_path, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_conversation(&row))
            .collect()
    }

    pub async fn touch(&self, id: i64) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn update_title(&self, id: i64, title: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(ts)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

/// Repository for managing summarised conversation context.
#[derive(Debug)]
pub struct ConversationSummaryRepository {
    database: Arc<DatabaseManager>,
}

impl ConversationSummaryRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn upsert(
        &self,
        conversation_id: i64,
        summary_content: &str,
        summary_tokens: i64,
        messages_before_summary: i64,
        tokens_saved: i64,
        compression_cost: f64,
    ) -> AgentResult<ConversationSummary> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO conversation_summaries (
                conversation_id, summary_content, summary_tokens,
                messages_before_summary, tokens_saved, compression_cost, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(conversation_id) DO UPDATE SET
                summary_content = excluded.summary_content,
                summary_tokens = excluded.summary_tokens,
                messages_before_summary = excluded.messages_before_summary,
                tokens_saved = excluded.tokens_saved,
                compression_cost = excluded.compression_cost,
                updated_at = excluded.updated_at",
        )
        .bind(conversation_id)
        .bind(summary_content)
        .bind(summary_tokens)
        .bind(messages_before_summary)
        .bind(tokens_saved)
        .bind(compression_cost)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get(conversation_id).await?.ok_or_else(|| {
            AgentError::Internal(format!(
                "Failed to retrieve conversation summary: {}",
                conversation_id
            ))
        })
    }

    pub async fn get(&self, conversation_id: i64) -> AgentResult<Option<ConversationSummary>> {
        let row = sqlx::query(
            "SELECT conversation_id, summary_content, summary_tokens,
                    messages_before_summary, tokens_saved, compression_cost,
                    created_at, updated_at
             FROM conversation_summaries WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_conversation_summary(&row)).transpose()
    }

    pub async fn delete(&self, conversation_id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM conversation_summaries WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

/// Repository that tracks per-file context state.
#[derive(Debug)]
pub struct FileContextRepository {
    database: Arc<DatabaseManager>,
}

impl FileContextRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_entry(
        &self,
        conversation_id: i64,
        file_path: &str,
        record_state: FileRecordState,
        record_source: FileRecordSource,
        agent_read_timestamp: Option<DateTime<Utc>>,
        agent_edit_timestamp: Option<DateTime<Utc>>,
        user_edit_timestamp: Option<DateTime<Utc>>,
    ) -> AgentResult<FileContextEntry> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO file_context_entries (
                conversation_id, file_path, record_state, record_source,
                agent_read_timestamp, agent_edit_timestamp, user_edit_timestamp, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(conversation_id, file_path) DO UPDATE SET
                record_state = excluded.record_state,
                record_source = excluded.record_source,
                agent_read_timestamp = excluded.agent_read_timestamp,
                agent_edit_timestamp = excluded.agent_edit_timestamp,
                user_edit_timestamp = excluded.user_edit_timestamp",
        )
        .bind(conversation_id)
        .bind(file_path)
        .bind(record_state.as_str())
        .bind(record_source.as_str())
        .bind(opt_datetime_to_timestamp(agent_read_timestamp))
        .bind(opt_datetime_to_timestamp(agent_edit_timestamp))
        .bind(opt_datetime_to_timestamp(user_edit_timestamp))
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.find_by_path(conversation_id, file_path)
            .await?
            .ok_or_else(|| {
                AgentError::Internal(format!(
                    "Failed to retrieve file context entry: {}",
                    file_path
                ))
            })
    }

    pub async fn find_by_path(
        &self,
        conversation_id: i64,
        file_path: &str,
    ) -> AgentResult<Option<FileContextEntry>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, file_path, record_state, record_source,
                    agent_read_timestamp, agent_edit_timestamp, user_edit_timestamp, created_at
             FROM file_context_entries WHERE conversation_id = ? AND file_path = ?",
        )
        .bind(conversation_id)
        .bind(file_path)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_file_context_entry(&row)).transpose()
    }

    pub async fn get_active_files(
        &self,
        conversation_id: i64,
    ) -> AgentResult<Vec<FileContextEntry>> {
        self.list_by_state(conversation_id, FileRecordState::Active)
            .await
    }

    pub async fn get_stale_files(
        &self,
        conversation_id: i64,
    ) -> AgentResult<Vec<FileContextEntry>> {
        self.list_by_state(conversation_id, FileRecordState::Stale)
            .await
    }

    pub async fn list_by_state(
        &self,
        conversation_id: i64,
        state: FileRecordState,
    ) -> AgentResult<Vec<FileContextEntry>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, file_path, record_state, record_source,
                    agent_read_timestamp, agent_edit_timestamp, user_edit_timestamp, created_at
             FROM file_context_entries
             WHERE conversation_id = ? AND record_state = ?
             ORDER BY created_at DESC",
        )
        .bind(conversation_id)
        .bind(state.as_str())
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_file_context_entry(&row))
            .collect()
    }

    pub async fn mark_file_state(
        &self,
        conversation_id: i64,
        file_path: &str,
        state: FileRecordState,
    ) -> AgentResult<()> {
        sqlx::query(
            "UPDATE file_context_entries SET record_state = ?, created_at = created_at
             WHERE conversation_id = ? AND file_path = ?",
        )
        .bind(state.as_str())
        .bind(conversation_id)
        .bind(file_path)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn delete_stale_entries_before(
        &self,
        conversation_id: i64,
        cutoff: DateTime<Utc>,
    ) -> AgentResult<u64> {
        let result = sqlx::query(
            "DELETE FROM file_context_entries
             WHERE conversation_id = ? AND record_state = 'stale' AND created_at < ?",
        )
        .bind(conversation_id)
        .bind(datetime_to_timestamp(cutoff))
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_for_conversation(&self, conversation_id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM file_context_entries WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// 获取被 Agent 编辑过的文件（有 agent_edit_timestamp 的文件）
    pub async fn get_agent_edited_files(
        &self,
        conversation_id: i64,
    ) -> AgentResult<Vec<FileContextEntry>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, file_path, record_state, record_source,
                    agent_read_timestamp, agent_edit_timestamp, user_edit_timestamp, created_at
             FROM file_context_entries
             WHERE conversation_id = ? AND agent_edit_timestamp IS NOT NULL
             ORDER BY agent_edit_timestamp DESC",
        )
        .bind(conversation_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_file_context_entry(&row))
            .collect()
    }
}

/// Repository for agent execution records.
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
        conversation_id: i64,
        user_request: &str,
        system_prompt_used: &str,
        execution_config: Option<&str>,
        has_conversation_context: bool,
        max_iterations: i64,
    ) -> AgentResult<AgentExecution> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO agent_executions (
                execution_id, conversation_id, user_request, system_prompt_used, execution_config,
                has_conversation_context, status, current_iteration, error_count, max_iterations,
                total_input_tokens, total_output_tokens, total_cost, context_tokens,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, 'running', 0, 0, ?, 0, 0, 0.0, 0, ?, ?)",
        )
        .bind(execution_id)
        .bind(conversation_id)
        .bind(user_request)
        .bind(system_prompt_used)
        .bind(execution_config)
        .bind(bool_to_sql(has_conversation_context))
        .bind(max_iterations)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get_by_execution_id(&execution_id)
            .await?
            .ok_or_else(|| {
                AgentError::Internal("Failed to create agent execution record".to_string())
            })
    }

    pub async fn get(&self, id: i64) -> AgentResult<Option<AgentExecution>> {
        let row = sqlx::query("SELECT * FROM agent_executions WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;

        row.map(|row| build_agent_execution(&row)).transpose()
    }

    pub async fn get_by_execution_id(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<AgentExecution>> {
        let row = sqlx::query("SELECT * FROM agent_executions WHERE execution_id = ?")
            .bind(execution_id)
            .fetch_optional(self.pool())
            .await?;

        row.map(|row| build_agent_execution(&row)).transpose()
    }

    pub async fn list_recent_by_conversation(
        &self,
        conversation_id: i64,
        limit: i64,
    ) -> AgentResult<Vec<AgentExecution>> {
        let rows = sqlx::query(
            "SELECT * FROM agent_executions WHERE conversation_id = ?
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_agent_execution(&row))
            .collect()
    }

    pub async fn list_recent(&self, limit: i64) -> AgentResult<Vec<AgentExecution>> {
        let rows = sqlx::query("SELECT * FROM agent_executions ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(self.pool())
            .await?;

        rows.into_iter()
            .map(|row| build_agent_execution(&row))
            .collect()
    }

    pub async fn mark_started(&self, execution_id: &str) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "UPDATE agent_executions SET started_at = ?, status = 'running', updated_at = ?
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
            "UPDATE agent_executions SET status = ?, current_iteration = ?, error_count = ?, updated_at = ?
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
            "UPDATE agent_executions SET has_conversation_context = ?, updated_at = ?
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
        execution_config: Option<&str>,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query("UPDATE agent_executions SET execution_config = ?, updated_at = ? WHERE execution_id = ?")
            .bind(execution_config)
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
            "UPDATE agent_executions SET status = ?, completed_at = ?, updated_at = ?, current_iteration = current_iteration
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
            "UPDATE agent_executions SET
                total_input_tokens = ?,
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

    pub async fn aggregate_token_usage(
        &self,
        conversation_id: i64,
    ) -> AgentResult<TokenUsageStats> {
        let row = sqlx::query(
            "SELECT
                COALESCE(SUM(total_input_tokens), 0) AS total_input_tokens,
                COALESCE(SUM(total_output_tokens), 0) AS total_output_tokens,
                COALESCE(SUM(context_tokens), 0) AS total_context_tokens,
                COALESCE(SUM(total_cost), 0.0) AS total_cost
             FROM agent_executions WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_one(self.pool())
        .await?;

        Ok(TokenUsageStats::new(
            row.try_get("total_input_tokens")?,
            row.try_get("total_output_tokens")?,
            row.try_get("total_context_tokens")?,
            row.try_get("total_cost")?,
        ))
    }

    pub async fn delete_by_conversation(&self, conversation_id: i64) -> AgentResult<()> {
        sqlx::query("DELETE FROM agent_executions WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_after(
        &self,
        conversation_id: i64,
        cutoff_timestamp: i64,
    ) -> AgentResult<()> {
        sqlx::query(
            "DELETE FROM agent_executions
             WHERE conversation_id = ? AND created_at >= ?",
        )
        .bind(conversation_id)
        .bind(cutoff_timestamp)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

/// Repository for execution messages.
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
        role: MessageRole,
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
            .ok_or_else(|| AgentError::Internal("Failed to insert execution message".to_string()))
    }

    pub async fn latest_for_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<ExecutionMessage>> {
        let row = sqlx::query(
            "SELECT * FROM execution_messages WHERE execution_id = ?
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_execution_message(&row)).transpose()
    }

    pub async fn list_by_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Vec<ExecutionMessage>> {
        let rows = sqlx::query(
            "SELECT * FROM execution_messages WHERE execution_id = ?
             ORDER BY iteration ASC, sequence ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_execution_message(&row))
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

/// Repository for tool execution history.
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
            "UPDATE tool_executions SET
                status = ?,
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
            "SELECT * FROM tool_executions WHERE execution_id = ?
             ORDER BY started_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_tool_execution(&row)).transpose()
    }

    pub async fn list_by_execution(&self, execution_id: &str) -> AgentResult<Vec<ToolExecution>> {
        let rows = sqlx::query(
            "SELECT * FROM tool_executions WHERE execution_id = ?
             ORDER BY started_at ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_tool_execution(&row))
            .collect()
    }

    pub async fn delete_for_execution(&self, execution_id: &str) -> AgentResult<()> {
        sqlx::query("DELETE FROM tool_executions WHERE execution_id = ?")
            .bind(execution_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

/// Repository for fine-grained execution events.
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
            .ok_or_else(|| AgentError::Internal("Failed to record event".to_string()))
    }

    pub async fn list_by_execution(&self, execution_id: &str) -> AgentResult<Vec<ExecutionEvent>> {
        let rows = sqlx::query(
            "SELECT * FROM execution_events WHERE execution_id = ?
             ORDER BY iteration ASC, created_at ASC",
        )
        .bind(execution_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| build_execution_event(&row))
            .collect()
    }

    pub async fn last_for_execution(
        &self,
        execution_id: &str,
    ) -> AgentResult<Option<ExecutionEvent>> {
        let row = sqlx::query(
            "SELECT * FROM execution_events WHERE execution_id = ?
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| build_execution_event(&row)).transpose()
    }

    pub async fn delete_for_execution(&self, execution_id: &str) -> AgentResult<()> {
        sqlx::query("DELETE FROM execution_events WHERE execution_id = ?")
            .bind(execution_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
