use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppResult;

use super::{opt_timestamp_to_datetime, timestamp_to_datetime};
use sqlx::Row;

/// Top-level conversation container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub title: Option<String>,
    pub workspace_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(id: i64, title: Option<String>, workspace_path: Option<String>, ts: i64) -> Self {
        let created = timestamp_to_datetime(ts);
        Self {
            id,
            title,
            workspace_path,
            created_at: created,
            updated_at: created,
        }
    }
}

/// Conversation summary entity that keeps track of compression metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: i64,
    pub summary_content: String,
    pub summary_tokens: i64,
    pub messages_before_summary: i64,
    pub tokens_saved: i64,
    pub compression_cost: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRecordState {
    Active,
    Stale,
}

impl FileRecordState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stale => "stale",
        }
    }
}

impl FromStr for FileRecordState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "stale" => Ok(Self::Stale),
            other => Err(anyhow!("未知的文件状态: {}", other)),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRecordSource {
    ReadTool,
    UserEdited,
    AgentEdited,
    FileMentioned,
}

impl FileRecordSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadTool => "read_tool",
            Self::UserEdited => "user_edited",
            Self::AgentEdited => "agent_edited",
            Self::FileMentioned => "file_mentioned",
        }
    }
}

impl FromStr for FileRecordSource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read_tool" => Ok(Self::ReadTool),
            "user_edited" => Ok(Self::UserEdited),
            "agent_edited" => Ok(Self::AgentEdited),
            "file_mentioned" => Ok(Self::FileMentioned),
            other => Err(anyhow!("未知的文件来源: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContextEntry {
    pub id: i64,
    pub conversation_id: i64,
    pub file_path: String,
    pub record_state: FileRecordState,
    pub record_source: FileRecordSource,
    pub agent_read_timestamp: Option<DateTime<Utc>>,
    pub agent_edit_timestamp: Option<DateTime<Utc>>,
    pub user_edit_timestamp: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl FileContextEntry {
    pub fn new(
        id: i64,
        conversation_id: i64,
        file_path: String,
        record_state: FileRecordState,
        record_source: FileRecordSource,
        agent_read_timestamp: Option<i64>,
        agent_edit_timestamp: Option<i64>,
        user_edit_timestamp: Option<i64>,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            conversation_id,
            file_path,
            record_state,
            record_source,
            agent_read_timestamp: opt_timestamp_to_datetime(agent_read_timestamp),
            agent_edit_timestamp: opt_timestamp_to_datetime(agent_edit_timestamp),
            user_edit_timestamp: opt_timestamp_to_datetime(user_edit_timestamp),
            created_at: timestamp_to_datetime(created_at),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Error,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for ExecutionStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(anyhow!("未知的执行状态: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    pub id: i64,
    pub execution_id: String,
    pub conversation_id: i64,
    pub user_request: String,
    pub system_prompt_used: String,
    pub execution_config: Option<String>,
    pub has_conversation_context: bool,
    pub status: ExecutionStatus,
    pub current_iteration: i64,
    pub error_count: i64,
    pub max_iterations: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
    pub context_tokens: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentExecution {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        execution_id: String,
        conversation_id: i64,
        user_request: String,
        system_prompt_used: String,
        execution_config: Option<String>,
        has_conversation_context: bool,
        status: ExecutionStatus,
        current_iteration: i64,
        error_count: i64,
        max_iterations: i64,
        total_input_tokens: i64,
        total_output_tokens: i64,
        total_cost: f64,
        context_tokens: i64,
        created_at: i64,
        updated_at: i64,
        started_at: Option<i64>,
        completed_at: Option<i64>,
    ) -> Self {
        Self {
            id,
            execution_id,
            conversation_id,
            user_request,
            system_prompt_used,
            execution_config,
            has_conversation_context,
            status,
            current_iteration,
            error_count,
            max_iterations,
            total_input_tokens,
            total_output_tokens,
            total_cost,
            context_tokens,
            created_at: timestamp_to_datetime(created_at),
            updated_at: timestamp_to_datetime(updated_at),
            started_at: opt_timestamp_to_datetime(started_at),
            completed_at: opt_timestamp_to_datetime(completed_at),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

impl FromStr for MessageRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            other => Err(anyhow!("未知的消息角色: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMessage {
    pub id: i64,
    pub execution_id: String,
    pub role: MessageRole,
    pub content: String,
    pub tokens: i64,
    pub is_summary: bool,
    pub iteration: i64,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
}

impl ExecutionMessage {
    pub fn new(
        id: i64,
        execution_id: String,
        role: MessageRole,
        content: String,
        tokens: i64,
        is_summary: bool,
        iteration: i64,
        sequence: i64,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            execution_id,
            role,
            content,
            tokens,
            is_summary,
            iteration,
            sequence,
            created_at: timestamp_to_datetime(created_at),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolExecutionStatus {
    Pending,
    Running,
    Completed,
    Error,
}

impl ToolExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
        }
    }
}

impl FromStr for ToolExecutionStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            other => Err(anyhow!("未知的工具执行状态: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: i64,
    pub execution_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_arguments: String,
    pub tool_result: Option<String>,
    pub error_message: Option<String>,
    pub status: ToolExecutionStatus,
    pub files_read: String,
    pub files_written: String,
    pub directories_accessed: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

impl ToolExecution {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        execution_id: String,
        tool_call_id: String,
        tool_name: String,
        tool_arguments: String,
        tool_result: Option<String>,
        error_message: Option<String>,
        status: ToolExecutionStatus,
        files_read: String,
        files_written: String,
        directories_accessed: String,
        started_at: i64,
        completed_at: Option<i64>,
        duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id,
            execution_id,
            tool_call_id,
            tool_name,
            tool_arguments,
            tool_result,
            error_message,
            status,
            files_read,
            files_written,
            directories_accessed,
            started_at: timestamp_to_datetime(started_at),
            completed_at: opt_timestamp_to_datetime(completed_at),
            duration_ms,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionEventType {
    Thinking,
    Text,
    ToolCall,
    ToolResult,
    Error,
    Finish,
}

impl ExecutionEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Thinking => "thinking",
            Self::Text => "text",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::Error => "error",
            Self::Finish => "finish",
        }
    }
}

impl FromStr for ExecutionEventType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "thinking" => Ok(Self::Thinking),
            "text" => Ok(Self::Text),
            "tool_call" => Ok(Self::ToolCall),
            "tool_result" => Ok(Self::ToolResult),
            "error" => Ok(Self::Error),
            "finish" => Ok(Self::Finish),
            other => Err(anyhow!("未知的事件类型: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub id: i64,
    pub execution_id: String,
    pub event_type: ExecutionEventType,
    pub event_data: String,
    pub iteration: i64,
    pub created_at: DateTime<Utc>,
}

impl ExecutionEvent {
    pub fn new(
        id: i64,
        execution_id: String,
        event_type: ExecutionEventType,
        event_data: String,
        iteration: i64,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            execution_id,
            event_type,
            event_data,
            iteration,
            created_at: timestamp_to_datetime(created_at),
        }
    }
}

/// Aggregated token usage for a conversation or execution scope.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenUsageStats {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_context_tokens: i64,
    pub total_cost: f64,
}

impl TokenUsageStats {
    pub fn new(
        total_input_tokens: i64,
        total_output_tokens: i64,
        total_context_tokens: i64,
        total_cost: f64,
    ) -> Self {
        Self {
            total_input_tokens,
            total_output_tokens,
            total_context_tokens,
            total_cost,
        }
    }
}

/// Helper constructors to materialise domain objects from raw database rows.
pub(crate) fn build_conversation(row: &sqlx::sqlite::SqliteRow) -> AppResult<Conversation> {
    Ok(Conversation {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        workspace_path: row.try_get("workspace_path")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
    })
}

pub(crate) fn build_conversation_summary(
    row: &sqlx::sqlite::SqliteRow,
) -> AppResult<ConversationSummary> {
    Ok(ConversationSummary {
        conversation_id: row.try_get("conversation_id")?,
        summary_content: row.try_get("summary_content")?,
        summary_tokens: row.try_get("summary_tokens")?,
        messages_before_summary: row.try_get("messages_before_summary")?,
        tokens_saved: row.try_get("tokens_saved")?,
        compression_cost: row.try_get("compression_cost")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
    })
}

pub(crate) fn build_file_context_entry(
    row: &sqlx::sqlite::SqliteRow,
) -> AppResult<FileContextEntry> {
    Ok(FileContextEntry::new(
        row.try_get("id")?,
        row.try_get("conversation_id")?,
        row.try_get("file_path")?,
        FileRecordState::from_str(&row.try_get::<String, _>("record_state")?)?,
        FileRecordSource::from_str(&row.try_get::<String, _>("record_source")?)?,
        row.try_get("agent_read_timestamp")?,
        row.try_get("agent_edit_timestamp")?,
        row.try_get("user_edit_timestamp")?,
        row.try_get("created_at")?,
    ))
}

pub(crate) fn build_agent_execution(row: &sqlx::sqlite::SqliteRow) -> AppResult<AgentExecution> {
    Ok(AgentExecution::new(
        row.try_get("id")?,
        row.try_get("execution_id")?,
        row.try_get("conversation_id")?,
        row.try_get("user_request")?,
        row.try_get("system_prompt_used")?,
        row.try_get("execution_config")?,
        super::sql_to_bool(row.try_get::<i64, _>("has_conversation_context")?),
        ExecutionStatus::from_str(&row.try_get::<String, _>("status")?)?,
        row.try_get("current_iteration")?,
        row.try_get("error_count")?,
        row.try_get("max_iterations")?,
        row.try_get("total_input_tokens")?,
        row.try_get("total_output_tokens")?,
        row.try_get("total_cost")?,
        row.try_get("context_tokens")?,
        row.try_get("created_at")?,
        row.try_get("updated_at")?,
        row.try_get("started_at")?,
        row.try_get("completed_at")?,
    ))
}

pub(crate) fn build_execution_message(
    row: &sqlx::sqlite::SqliteRow,
) -> AppResult<ExecutionMessage> {
    Ok(ExecutionMessage::new(
        row.try_get("id")?,
        row.try_get("execution_id")?,
        MessageRole::from_str(&row.try_get::<String, _>("role")?)?,
        row.try_get("content")?,
        row.try_get("tokens")?,
        super::sql_to_bool(row.try_get::<i64, _>("is_summary")?),
        row.try_get("iteration")?,
        row.try_get("sequence")?,
        row.try_get("created_at")?,
    ))
}

pub(crate) fn build_tool_execution(row: &sqlx::sqlite::SqliteRow) -> AppResult<ToolExecution> {
    Ok(ToolExecution::new(
        row.try_get("id")?,
        row.try_get("execution_id")?,
        row.try_get("tool_call_id")?,
        row.try_get("tool_name")?,
        row.try_get("tool_arguments")?,
        row.try_get("tool_result")?,
        row.try_get("error_message")?,
        ToolExecutionStatus::from_str(&row.try_get::<String, _>("status")?)?,
        row.try_get("files_read")?,
        row.try_get("files_written")?,
        row.try_get("directories_accessed")?,
        row.try_get("started_at")?,
        row.try_get("completed_at")?,
        row.try_get("duration_ms")?,
    ))
}

pub(crate) fn build_execution_event(row: &sqlx::sqlite::SqliteRow) -> AppResult<ExecutionEvent> {
    Ok(ExecutionEvent::new(
        row.try_get("id")?,
        row.try_get("execution_id")?,
        ExecutionEventType::from_str(&row.try_get::<String, _>("event_type")?)?,
        row.try_get("event_data")?,
        row.try_get("iteration")?,
        row.try_get("created_at")?,
    ))
}
