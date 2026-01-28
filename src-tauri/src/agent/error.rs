/*!
 * Agent 模块统一错误定义（集中式）
 * 所有 Agent 相关的错误类型都定义在此文件中
 */

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::error::RepositoryError;

// ==================== 顶层聚合错误 ====================

#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    TaskExecutor(#[from] TaskExecutorError),
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

// ==================== TaskExecutor 子模块错误 ====================

#[derive(Error, Debug)]
pub enum TaskExecutorError {
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

    #[error("Invalid task state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Internal task executor error: {0}")]
    InternalError(String),
}

impl TaskExecutorError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            TaskExecutorError::TaskNotFound(_) => false,
            TaskExecutorError::TaskAlreadyCompleted(_) => false,
            TaskExecutorError::TaskCancelled(_) => false,
            TaskExecutorError::MaxIterationsReached { .. } => false,
            TaskExecutorError::TooManyErrors { .. } => false,
            TaskExecutorError::LLMCallFailed(_) => true,
            TaskExecutorError::ToolExecutionFailed { .. } => true,
            TaskExecutorError::StatePersistenceFailed(_) => true,
            TaskExecutorError::ContextRecoveryFailed(_) => false,
            TaskExecutorError::ChannelError(_) => true,
            TaskExecutorError::ConfigurationError(_) => false,
            TaskExecutorError::JsonError(_) => false,
            TaskExecutorError::DatabaseError(_) => true,
            TaskExecutorError::RepositoryError(_) => true,
            TaskExecutorError::TaskInterrupted => true,
            TaskExecutorError::InvalidStateTransition { .. } => false,
            TaskExecutorError::InternalError(_) => false,
        }
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TaskExecutorError::TaskNotFound(_) => ErrorSeverity::Warning,
            TaskExecutorError::TaskAlreadyCompleted(_) => ErrorSeverity::Info,
            TaskExecutorError::TaskCancelled(_) => ErrorSeverity::Info,
            TaskExecutorError::MaxIterationsReached { .. } => ErrorSeverity::Warning,
            TaskExecutorError::TooManyErrors { .. } => ErrorSeverity::Error,
            TaskExecutorError::LLMCallFailed(_) => ErrorSeverity::Error,
            TaskExecutorError::ToolExecutionFailed { .. } => ErrorSeverity::Warning,
            TaskExecutorError::StatePersistenceFailed(_) => ErrorSeverity::Error,
            TaskExecutorError::ContextRecoveryFailed(_) => ErrorSeverity::Error,
            TaskExecutorError::ChannelError(_) => ErrorSeverity::Warning,
            TaskExecutorError::ConfigurationError(_) => ErrorSeverity::Error,
            TaskExecutorError::JsonError(_) => ErrorSeverity::Error,
            TaskExecutorError::DatabaseError(_) => ErrorSeverity::Error,
            TaskExecutorError::RepositoryError(_) => ErrorSeverity::Error,
            TaskExecutorError::TaskInterrupted => ErrorSeverity::Info,
            TaskExecutorError::InvalidStateTransition { .. } => ErrorSeverity::Error,
            TaskExecutorError::InternalError(_) => ErrorSeverity::Critical,
        }
    }
}

pub type TaskExecutorResult<T> = Result<T, TaskExecutorError>;

// ==================== ToolExecutor 子模块错误 ====================

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

impl From<AgentError> for TaskExecutorError {
    fn from(error: AgentError) -> Self {
        match error {
            AgentError::TaskExecutor(e) => e,
            AgentError::ToolExecutor(e) => TaskExecutorError::ToolExecutionFailed {
                tool_name: "unknown".to_string(),
                error: e.to_string(),
            },
            AgentError::Repository(e) => TaskExecutorError::RepositoryError(e),
            AgentError::Json(e) => TaskExecutorError::JsonError(e),
            AgentError::Database(e) => TaskExecutorError::DatabaseError(e.to_string()),
            AgentError::Io(e) => TaskExecutorError::InternalError(e.to_string()),
            AgentError::Utf8(e) => TaskExecutorError::InternalError(e.to_string()),
            AgentError::XmlParse(e)
            | AgentError::XmlSerialize(e)
            | AgentError::TemplateRender(e)
            | AgentError::Parse(e)
            | AgentError::Internal(e)
            | AgentError::InvalidSkillFormat(e)
            | AgentError::SkillNotFound(e) => TaskExecutorError::InternalError(e),
        }
    }
}

impl From<AgentError> for ToolExecutorError {
    fn from(error: AgentError) -> Self {
        match error {
            AgentError::ToolExecutor(e) => e,
            AgentError::Json(e) => ToolExecutorError::JsonError(e),
            AgentError::Io(e) => ToolExecutorError::IoError(e),
            AgentError::TaskExecutor(e) => ToolExecutorError::InternalError(e.to_string()),
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

// ==================== 共用类型 ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// ==================== 兼容性：前端使用的类型 ====================

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
