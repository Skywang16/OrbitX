/*!
 * Agent module unified error definitions (centralized)
 * All Agent-related error types are defined in this file
 */

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::error::RepositoryError;

// ==================== Top-level aggregate errors ====================

#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    AgentRunExecutor(#[from] AgentRunError),
    #[error(transparent)]
    ToolExecutor(#[from] ToolExecutorError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("XML parse error: {0}")]
    XmlParse(String),
    #[error("XML serialization error: {0}")]
    XmlSerialize(String),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Template render error: {0}")]
    TemplateRender(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Agent internal error: {0}")]
    Internal(String),
    #[error("Invalid skill format: {0}")]
    InvalidSkillFormat(String),
    #[error("Skill not found: {0}")]
    SkillNotFound(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

impl From<xmltree::ParseError> for AgentError {
    fn from(err: xmltree::ParseError) -> Self {
        AgentError::XmlParse(err.to_string())
    }
}

impl From<xmltree::Error> for AgentError {
    fn from(err: xmltree::Error) -> Self {
        AgentError::XmlSerialize(err.to_string())
    }
}

// ==================== AgentRunExecutor submodule errors ====================

#[derive(Error, Debug)]
pub enum AgentRunError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already completed: {0}")]
    TaskAlreadyCompleted(String),

    #[error("Task cancelled: {0}")]
    TaskCancelled(String),

    #[error("Maximum iteration limit reached: {current}/{max}")]
    MaxIterationsReached { current: u32, max: u32 },

    #[error("Maximum error count reached: {error_count}")]
    TooManyErrors { error_count: u32 },

    #[error("LLM invocation failed: {0}")]
    LLMCallFailed(String),

    #[error("Tool execution failed: {tool_name}: {error}")]
    ToolExecutionFailed { tool_name: String, error: String },

    #[error("State persistence failed: {0}")]
    StatePersistenceFailed(String),

    #[error("Context recovery failed: {0}")]
    ContextRecoveryFailed(String),

    #[error("Channel communication failed: {0}")]
    ChannelError(#[from] tauri::Error),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error("Repository operation failed: {0}")]
    RepositoryError(#[from] RepositoryError),

    #[error("Task execution interrupted")]
    TaskInterrupted,

    #[error("Too many active tasks globally: {current}/{limit}")]
    TooManyActiveTasksGlobal { current: usize, limit: usize },

    #[error("Too many active subagents globally: {current}/{limit}")]
    TooManyActiveSubagentsGlobal { current: usize, limit: usize },

    #[error("Too many active subagents for parent task {parent_task_id}: {current}/{limit}")]
    TooManyActiveSubagentsPerParent {
        parent_task_id: String,
        current: usize,
        limit: usize,
    },

    #[error("Invalid task state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Internal task executor error: {0}")]
    InternalError(String),
}

impl AgentRunError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            AgentRunError::TaskNotFound(_) => false,
            AgentRunError::TaskAlreadyCompleted(_) => false,
            AgentRunError::TaskCancelled(_) => false,
            AgentRunError::MaxIterationsReached { .. } => false,
            AgentRunError::TooManyErrors { .. } => false,
            AgentRunError::LLMCallFailed(_) => true,
            AgentRunError::ToolExecutionFailed { .. } => true,
            AgentRunError::StatePersistenceFailed(_) => true,
            AgentRunError::ContextRecoveryFailed(_) => false,
            AgentRunError::ChannelError(_) => true,
            AgentRunError::ConfigurationError(_) => false,
            AgentRunError::JsonError(_) => false,
            AgentRunError::DatabaseError(_) => true,
            AgentRunError::RepositoryError(_) => true,
            AgentRunError::TaskInterrupted => true,
            AgentRunError::TooManyActiveTasksGlobal { .. } => false,
            AgentRunError::TooManyActiveSubagentsGlobal { .. } => false,
            AgentRunError::TooManyActiveSubagentsPerParent { .. } => false,
            AgentRunError::InvalidStateTransition { .. } => false,
            AgentRunError::InternalError(_) => false,
        }
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AgentRunError::TaskNotFound(_) => ErrorSeverity::Warning,
            AgentRunError::TaskAlreadyCompleted(_) => ErrorSeverity::Info,
            AgentRunError::TaskCancelled(_) => ErrorSeverity::Info,
            AgentRunError::MaxIterationsReached { .. } => ErrorSeverity::Warning,
            AgentRunError::TooManyErrors { .. } => ErrorSeverity::Error,
            AgentRunError::LLMCallFailed(_) => ErrorSeverity::Error,
            AgentRunError::ToolExecutionFailed { .. } => ErrorSeverity::Warning,
            AgentRunError::StatePersistenceFailed(_) => ErrorSeverity::Error,
            AgentRunError::ContextRecoveryFailed(_) => ErrorSeverity::Error,
            AgentRunError::ChannelError(_) => ErrorSeverity::Warning,
            AgentRunError::ConfigurationError(_) => ErrorSeverity::Error,
            AgentRunError::JsonError(_) => ErrorSeverity::Error,
            AgentRunError::DatabaseError(_) => ErrorSeverity::Error,
            AgentRunError::RepositoryError(_) => ErrorSeverity::Error,
            AgentRunError::TaskInterrupted => ErrorSeverity::Info,
            AgentRunError::TooManyActiveTasksGlobal { .. } => ErrorSeverity::Warning,
            AgentRunError::TooManyActiveSubagentsGlobal { .. } => ErrorSeverity::Warning,
            AgentRunError::TooManyActiveSubagentsPerParent { .. } => ErrorSeverity::Warning,
            AgentRunError::InvalidStateTransition { .. } => ErrorSeverity::Error,
            AgentRunError::InternalError(_) => ErrorSeverity::Critical,
        }
    }
}

pub type AgentRunResult<T> = Result<T, AgentRunError>;

// ==================== ToolExecutor submodule errors ====================

#[derive(Error, Debug)]
pub enum ToolExecutorError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid arguments for tool {tool_name}: {error}")]
    InvalidArguments { tool_name: String, error: String },

    #[error("Tool execution failed: {tool_name}: {error}")]
    ExecutionFailed { tool_name: String, error: String },

    #[error("Permission denied: {tool_name} requires {required_permission}")]
    PermissionDenied {
        tool_name: String,
        required_permission: String,
    },

    #[error("Tool execution timed out: {tool_name} exceeded {timeout_seconds} seconds")]
    ExecutionTimeout {
        tool_name: String,
        timeout_seconds: u64,
    },

    #[error("Failed to parse tool result: {tool_name}: {error}")]
    ResultParsingFailed { tool_name: String, error: String },

    #[error("Tool resource limit exceeded: {tool_name}: {resource_type}")]
    ResourceLimitExceeded {
        tool_name: String,
        resource_type: String,
    },

    #[error("Tool invocation cycle detected: {call_chain}")]
    CircularDependency { call_chain: String },

    #[error("Tool configuration error: {0}")]
    ConfigurationError(String),

    #[error("Tool initialization failed: {tool_name}: {error}")]
    InitializationFailed { tool_name: String, error: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Internal tool executor error: {0}")]
    InternalError(String),
}

impl ToolExecutorError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            ToolExecutorError::ToolNotFound(_) => false,
            ToolExecutorError::InvalidArguments { .. } => false,
            ToolExecutorError::ExecutionFailed { .. } => true,
            ToolExecutorError::PermissionDenied { .. } => false,
            ToolExecutorError::ExecutionTimeout { .. } => true,
            ToolExecutorError::ResultParsingFailed { .. } => false,
            ToolExecutorError::ResourceLimitExceeded { .. } => true,
            ToolExecutorError::CircularDependency { .. } => false,
            ToolExecutorError::ConfigurationError(_) => false,
            ToolExecutorError::InitializationFailed { .. } => false,
            ToolExecutorError::IoError(_) => true,
            ToolExecutorError::JsonError(_) => false,
            ToolExecutorError::InternalError(_) => false,
        }
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ToolExecutorError::ToolNotFound(_) => ErrorSeverity::Warning,
            ToolExecutorError::InvalidArguments { .. } => ErrorSeverity::Warning,
            ToolExecutorError::ExecutionFailed { .. } => ErrorSeverity::Error,
            ToolExecutorError::PermissionDenied { .. } => ErrorSeverity::Warning,
            ToolExecutorError::ExecutionTimeout { .. } => ErrorSeverity::Warning,
            ToolExecutorError::ResultParsingFailed { .. } => ErrorSeverity::Error,
            ToolExecutorError::ResourceLimitExceeded { .. } => ErrorSeverity::Warning,
            ToolExecutorError::CircularDependency { .. } => ErrorSeverity::Error,
            ToolExecutorError::ConfigurationError(_) => ErrorSeverity::Error,
            ToolExecutorError::InitializationFailed { .. } => ErrorSeverity::Error,
            ToolExecutorError::IoError(_) => ErrorSeverity::Error,
            ToolExecutorError::JsonError(_) => ErrorSeverity::Error,
            ToolExecutorError::InternalError(_) => ErrorSeverity::Critical,
        }
    }
}

pub type ToolExecutorResult<T> = Result<T, ToolExecutorError>;

impl From<reqwest::Error> for ToolExecutorError {
    fn from(error: reqwest::Error) -> Self {
        ToolExecutorError::InternalError(error.to_string())
    }
}

impl From<AgentError> for AgentRunError {
    fn from(error: AgentError) -> Self {
        match error {
            AgentError::AgentRunExecutor(e) => e,
            AgentError::ToolExecutor(e) => AgentRunError::ToolExecutionFailed {
                tool_name: "unknown".to_string(),
                error: e.to_string(),
            },
            AgentError::Repository(e) => AgentRunError::RepositoryError(e),
            AgentError::Json(e) => AgentRunError::JsonError(e),
            AgentError::Database(e) => AgentRunError::DatabaseError(e.to_string()),
            AgentError::Io(e) => AgentRunError::InternalError(e.to_string()),
            AgentError::Utf8(e) => AgentRunError::InternalError(e.to_string()),
            AgentError::XmlParse(e)
            | AgentError::XmlSerialize(e)
            | AgentError::TemplateRender(e)
            | AgentError::Parse(e)
            | AgentError::Internal(e)
            | AgentError::InvalidSkillFormat(e)
            | AgentError::SkillNotFound(e) => AgentRunError::InternalError(e),
        }
    }
}

impl From<AgentError> for ToolExecutorError {
    fn from(error: AgentError) -> Self {
        match error {
            AgentError::ToolExecutor(e) => e,
            AgentError::Json(e) => ToolExecutorError::JsonError(e),
            AgentError::Io(e) => ToolExecutorError::IoError(e),
            AgentError::AgentRunExecutor(e) => ToolExecutorError::InternalError(e.to_string()),
            AgentError::Repository(e) => ToolExecutorError::InternalError(e.to_string()),
            AgentError::Database(e) => ToolExecutorError::InternalError(e.to_string()),
            AgentError::Utf8(e) => ToolExecutorError::InternalError(e.to_string()),
            AgentError::XmlParse(e)
            | AgentError::XmlSerialize(e)
            | AgentError::TemplateRender(e)
            | AgentError::Parse(e)
            | AgentError::Internal(e)
            | AgentError::InvalidSkillFormat(e)
            | AgentError::SkillNotFound(e) => ToolExecutorError::InternalError(e),
        }
    }
}

// ==================== Common types ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// ==================== Compatibility: types used by frontend ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorKind {
    TaskExecution,
    ToolExecution,
    ToolNotFound,
    LlmService,
    PromptBuilding,
    Context,
    Configuration,
    Database,
    Serialization,
    Channel,
    TaskNotFound,
    TaskAlreadyRunning,
    InvalidTaskState,
    MaxIterations,
    MaxErrors,
    InvalidArguments,
    PermissionDenied,
    Io,
    Json,
    Tauri,
    Unknown,
}

impl AgentErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentErrorKind::TaskExecution => "task_execution",
            AgentErrorKind::ToolExecution => "tool_execution",
            AgentErrorKind::ToolNotFound => "tool_not_found",
            AgentErrorKind::LlmService => "llm_service",
            AgentErrorKind::PromptBuilding => "prompt_building",
            AgentErrorKind::Context => "context",
            AgentErrorKind::Configuration => "configuration",
            AgentErrorKind::Database => "database",
            AgentErrorKind::Serialization => "serialization",
            AgentErrorKind::Channel => "channel",
            AgentErrorKind::TaskNotFound => "task_not_found",
            AgentErrorKind::TaskAlreadyRunning => "task_already_running",
            AgentErrorKind::InvalidTaskState => "invalid_task_state",
            AgentErrorKind::MaxIterations => "max_iterations",
            AgentErrorKind::MaxErrors => "max_errors",
            AgentErrorKind::InvalidArguments => "invalid_arguments",
            AgentErrorKind::PermissionDenied => "permission_denied",
            AgentErrorKind::Io => "io",
            AgentErrorKind::Json => "json",
            AgentErrorKind::Tauri => "tauri",
            AgentErrorKind::Unknown => "unknown",
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AgentErrorKind::LlmService
                | AgentErrorKind::ToolExecution
                | AgentErrorKind::Channel
                | AgentErrorKind::Io
                | AgentErrorKind::Database
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentErrorInfo {
    pub kind: AgentErrorKind,
    pub message: String,
    pub is_recoverable: bool,
}

impl AgentErrorInfo {
    pub fn new(kind: AgentErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        let is_recoverable = kind.is_recoverable();
        Self {
            kind,
            message,
            is_recoverable,
        }
    }
}
