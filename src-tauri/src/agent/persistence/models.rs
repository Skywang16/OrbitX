use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{self, Row};

use crate::agent::error::{AgentError, AgentResult};

use super::{opt_timestamp_to_datetime, timestamp_to_datetime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub path: String,
    pub display_name: Option<String>,
    pub active_session_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub workspace_path: String,
    pub title: Option<String>,
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
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "stale" => Ok(Self::Stale),
            other => Err(AgentError::Parse(format!("Unknown file state: {other}"))),
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
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read_tool" => Ok(Self::ReadTool),
            "user_edited" => Ok(Self::UserEdited),
            "agent_edited" => Ok(Self::AgentEdited),
            "file_mentioned" => Ok(Self::FileMentioned),
            other => Err(AgentError::Parse(format!(
                "Unknown file record source: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFileRecord {
    pub id: i64,
    pub workspace_path: String,
    pub relative_path: String,
    pub record_state: FileRecordState,
    pub record_source: FileRecordSource,
    pub agent_read_at: Option<DateTime<Utc>>,
    pub agent_edit_at: Option<DateTime<Utc>>,
    pub user_edit_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
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
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(AgentError::Parse(format!(
                "Unknown execution status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    pub id: i64,
    pub execution_id: String,
    pub session_id: i64,
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl FromStr for MessageRole {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "tool" => Ok(Self::Tool),
            other => Err(AgentError::Parse(format!(
                "Unknown message role: {other}"
            ))),
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
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            other => Err(AgentError::Parse(format!(
                "Unknown tool execution status: {other}"
            ))),
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
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "thinking" => Ok(Self::Thinking),
            "text" => Ok(Self::Text),
            "tool_call" => Ok(Self::ToolCall),
            "tool_result" => Ok(Self::ToolResult),
            "error" => Ok(Self::Error),
            "finish" => Ok(Self::Finish),
            other => Err(AgentError::Parse(format!("Unknown event type: {other}"))),
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenUsageStats {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_context_tokens: i64,
    pub total_cost: f64,
}

pub(crate) fn build_workspace(row: &sqlx::sqlite::SqliteRow) -> Workspace {
    Workspace {
        path: row.try_get("path").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or(None),
        active_session_id: row.try_get("active_session_id").unwrap_or(None),
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at").unwrap_or(0)),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at").unwrap_or(0)),
        last_accessed_at: timestamp_to_datetime(
            row.try_get::<i64, _>("last_accessed_at").unwrap_or(0),
        ),
    }
}

pub(crate) fn build_session(row: &sqlx::sqlite::SqliteRow) -> Session {
    Session {
        id: row.try_get("id").unwrap_or_default(),
        workspace_path: row.try_get("workspace_path").unwrap_or_default(),
        title: row.try_get("title").unwrap_or(None),
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at").unwrap_or(0)),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at").unwrap_or(0)),
    }
}

pub(crate) fn build_workspace_file_record(
    row: &sqlx::sqlite::SqliteRow,
) -> AgentResult<WorkspaceFileRecord> {
    Ok(WorkspaceFileRecord {
        id: row.try_get("id")?,
        workspace_path: row.try_get("workspace_path")?,
        relative_path: row.try_get("relative_path")?,
        record_state: FileRecordState::from_str(
            row.try_get::<String, _>("record_state")?.as_str(),
        )?,
        record_source: FileRecordSource::from_str(
            row.try_get::<String, _>("record_source")?.as_str(),
        )?,
        agent_read_at: opt_timestamp_to_datetime(row.try_get("agent_read_at")?),
        agent_edit_at: opt_timestamp_to_datetime(row.try_get("agent_edit_at")?),
        user_edit_at: opt_timestamp_to_datetime(row.try_get("user_edit_at")?),
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
    })
}

pub(crate) fn build_agent_execution(row: &sqlx::sqlite::SqliteRow) -> AgentResult<AgentExecution> {
    Ok(AgentExecution {
        id: row.try_get("id")?,
        execution_id: row.try_get("execution_id")?,
        session_id: row.try_get("session_id")?,
        user_request: row.try_get("user_request")?,
        system_prompt_used: row.try_get("system_prompt_used")?,
        execution_config: row.try_get("execution_config")?,
        has_conversation_context: row.try_get::<i64, _>("has_conversation_context")? != 0,
        status: ExecutionStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        current_iteration: row.try_get("current_iteration")?,
        error_count: row.try_get("error_count")?,
        max_iterations: row.try_get("max_iterations")?,
        total_input_tokens: row.try_get("total_input_tokens")?,
        total_output_tokens: row.try_get("total_output_tokens")?,
        total_cost: row.try_get("total_cost")?,
        context_tokens: row.try_get("context_tokens")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
        started_at: opt_timestamp_to_datetime(row.try_get("started_at")?),
        completed_at: opt_timestamp_to_datetime(row.try_get("completed_at")?),
    })
}

pub(crate) fn build_execution_message(
    row: &sqlx::sqlite::SqliteRow,
) -> AgentResult<ExecutionMessage> {
    Ok(ExecutionMessage {
        id: row.try_get("id")?,
        execution_id: row.try_get("execution_id")?,
        role: MessageRole::from_str(row.try_get::<String, _>("role")?.as_str())?,
        content: row.try_get("content")?,
        tokens: row.try_get("tokens")?,
        is_summary: row.try_get::<i64, _>("is_summary")? != 0,
        iteration: row.try_get("iteration")?,
        sequence: row.try_get("sequence")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
    })
}

pub(crate) fn build_tool_execution(row: &sqlx::sqlite::SqliteRow) -> AgentResult<ToolExecution> {
    Ok(ToolExecution {
        id: row.try_get("id")?,
        execution_id: row.try_get("execution_id")?,
        tool_call_id: row.try_get("tool_call_id")?,
        tool_name: row.try_get("tool_name")?,
        tool_arguments: row.try_get("tool_arguments")?,
        tool_result: row.try_get("tool_result")?,
        error_message: row.try_get("error_message")?,
        status: ToolExecutionStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        files_read: row.try_get("files_read")?,
        files_written: row.try_get("files_written")?,
        directories_accessed: row.try_get("directories_accessed")?,
        started_at: timestamp_to_datetime(row.try_get::<i64, _>("started_at")?),
        completed_at: opt_timestamp_to_datetime(row.try_get("completed_at")?),
        duration_ms: row.try_get("duration_ms")?,
    })
}

pub(crate) fn build_execution_event(row: &sqlx::sqlite::SqliteRow) -> AgentResult<ExecutionEvent> {
    Ok(ExecutionEvent {
        id: row.try_get("id")?,
        execution_id: row.try_get("execution_id")?,
        event_type: ExecutionEventType::from_str(row.try_get::<String, _>("event_type")?.as_str())?,
        event_data: row.try_get("event_data")?,
        iteration: row.try_get("iteration")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
    })
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
