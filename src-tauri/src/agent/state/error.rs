/*!
 * TaskExecutor错误处理（迁移至 agent/state）
 */

use anyhow::Result;
use thiserror::Error;

/// TaskExecutor错误类型
#[derive(Error, Debug)]
pub enum TaskExecutorError {
    #[error("任务不存在: {0}")]
    TaskNotFound(String),

    #[error("任务已完成，无法继续执行: {0}")]
    TaskAlreadyCompleted(String),

    #[error("任务已取消: {0}")]
    TaskCancelled(String),

    #[error("达到最大迭代次数限制: {current}/{max}")]
    MaxIterationsReached { current: u32, max: u32 },

    #[error("达到最大错误次数限制: {error_count}")]
    TooManyErrors { error_count: u32 },

    #[error("LLM调用失败: {0}")]
    LLMCallFailed(String),

    #[error("工具执行失败: {tool_name}: {error}")]
    ToolExecutionFailed { tool_name: String, error: String },

    #[error("状态持久化失败: {0}")]
    StatePersistenceFailed(String),

    #[error("上下文恢复失败: {0}")]
    ContextRecoveryFailed(String),

    #[error("Channel通信失败: {0}")]
    ChannelError(String),

    #[error("配置错误: {0}")]
    ConfigurationError(String),

    #[error("JSON序列化/反序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("数据库操作错误: {0}")]
    DatabaseError(String),

    #[error("任务执行被中断")]
    TaskInterrupted,

    #[error("无效的任务状态转换: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl TaskExecutorError {
    /// 判断错误是否可恢复
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
            TaskExecutorError::TaskInterrupted => true,
            TaskExecutorError::InvalidStateTransition { .. } => false,
            TaskExecutorError::InternalError(_) => false,
        }
    }

    /// 获取错误的严重级别
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
            TaskExecutorError::TaskInterrupted => ErrorSeverity::Info,
            TaskExecutorError::InvalidStateTransition { .. } => ErrorSeverity::Error,
            TaskExecutorError::InternalError(_) => ErrorSeverity::Critical,
        }
    }
}

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// TaskExecutor结果类型
pub type TaskExecutorResult<T> = Result<T>;

impl From<tauri::Error> for TaskExecutorError {
    fn from(error: tauri::Error) -> Self {
        TaskExecutorError::ChannelError(error.to_string())
    }
}
