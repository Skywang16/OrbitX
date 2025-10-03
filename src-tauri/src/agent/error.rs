use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::utils::error::{AppError, AppResult};

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

pub type AgentError = AppError;
pub type AgentResult<T> = AppResult<T>;

pub fn agent_error(kind: AgentErrorKind, message: impl Into<String>) -> AgentError {
    anyhow!("{}: {}", kind.as_str(), message.into())
}
