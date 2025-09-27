/*!
 * ToolExecutor错误处理（迁移至 agent/tools）
 */

use anyhow::Result;
use thiserror::Error;

/// ToolExecutor错误类型
#[derive(Error, Debug)]
pub enum ToolExecutorError {
    #[error("工具未找到: {0}")]
    ToolNotFound(String),

    #[error("工具参数无效: {tool_name}: {error}")]
    InvalidArguments { tool_name: String, error: String },

    #[error("工具执行失败: {tool_name}: {error}")]
    ExecutionFailed { tool_name: String, error: String },

    #[error("权限不足: {tool_name} 需要权限 {required_permission}")]
    PermissionDenied {
        tool_name: String,
        required_permission: String,
    },

    #[error("工具执行超时: {tool_name} 超过 {timeout_seconds}秒")]
    ExecutionTimeout {
        tool_name: String,
        timeout_seconds: u64,
    },

    #[error("工具返回值解析失败: {tool_name}: {error}")]
    ResultParsingFailed { tool_name: String, error: String },

    #[error("工具资源限制: {tool_name}: {resource_type}")]
    ResourceLimitExceeded {
        tool_name: String,
        resource_type: String,
    },

    #[error("工具调用循环依赖: {call_chain}")]
    CircularDependency { call_chain: String },

    #[error("工具配置错误: {0}")]
    ConfigurationError(String),

    #[error("工具初始化失败: {tool_name}: {error}")]
    InitializationFailed { tool_name: String, error: String },

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON序列化/反序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl ToolExecutorError {
    /// 判断错误是否可恢复
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

    /// 获取错误的严重级别
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

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// ToolExecutor结果类型
pub type ToolExecutorResult<T> = Result<T>;
