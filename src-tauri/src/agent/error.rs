use thiserror::Error;

/// Agent系统的错误类型
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Task execution error: {0}")]
    TaskExecutionError(String),

    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("LLM service error: {0}")]
    LLMServiceError(String),

    #[error("Prompt building error: {0}")]
    PromptBuildingError(String),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Channel communication error: {0}")]
    ChannelError(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already running: {0}")]
    TaskAlreadyRunning(String),

    #[error("Invalid task state: expected {expected}, got {actual}")]
    InvalidTaskState { expected: String, actual: String },

    #[error("Maximum iterations reached: {0}")]
    MaxIterationsReached(u32),

    #[error("Maximum errors reached: {0}")]
    MaxErrorsReached(u32),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),
}

impl AgentError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            AgentError::LLMServiceError(_) => true,
            AgentError::ToolExecutionError(_) => true,
            AgentError::ChannelError(_) => true,
            AgentError::IoError(_) => true,
            AgentError::MaxIterationsReached(_) => false,
            AgentError::MaxErrorsReached(_) => false,
            AgentError::TaskNotFound(_) => false,
            AgentError::ToolNotFound(_) => false,
            AgentError::InvalidTaskState { .. } => false,
            _ => true,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            AgentError::TaskExecutionError(_) => "task_execution",
            AgentError::ToolExecutionError(_) => "tool_execution",
            AgentError::ToolNotFound(_) => "tool_not_found",
            AgentError::LLMServiceError(_) => "llm_service",
            AgentError::PromptBuildingError(_) => "prompt_building",
            AgentError::ContextError(_) => "context",
            AgentError::ConfigurationError(_) => "configuration",
            AgentError::DatabaseError(_) => "database",
            AgentError::SerializationError(_) => "serialization",
            AgentError::ChannelError(_) => "channel",
            AgentError::TaskNotFound(_) => "task_not_found",
            AgentError::TaskAlreadyRunning(_) => "task_already_running",
            AgentError::InvalidTaskState { .. } => "invalid_task_state",
            AgentError::MaxIterationsReached(_) => "max_iterations",
            AgentError::MaxErrorsReached(_) => "max_errors",
            AgentError::InvalidArguments(_) => "invalid_arguments",
            AgentError::PermissionDenied(_) => "permission_denied",
            AgentError::IoError(_) => "io",
            AgentError::JsonError(_) => "json",
            AgentError::TauriError(_) => "tauri",
        }
    }
}

pub type AgentResult<T> = Result<T, AgentError>;
