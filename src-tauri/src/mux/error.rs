//! 终端系统统一错误处理
//!
//! 提供一致的错误类型定义、分级处理和转换机制

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

/// 终端系统统一错误类型
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum TerminalError {
    /// 配置错误
    #[error("配置错误: {message}")]
    Config { message: String },

    /// 面板操作错误
    #[error("面板操作错误: {operation} (面板ID: {pane_id}) - {message}")]
    Pane {
        operation: String,
        pane_id: u32,
        message: String,
    },

    /// Shell相关错误
    #[error("Shell错误: {operation} - {message}")]
    Shell { operation: String, message: String },

    /// I/O错误
    #[error("I/O错误: {operation} - {message}")]
    Io { operation: String, message: String },

    /// 并发错误（锁、同步等）
    #[error("并发错误: {operation} - {message}")]
    Concurrency { operation: String, message: String },

    /// 验证错误
    #[error("验证错误: {field} - {message}")]
    Validation { field: String, message: String },

    /// 系统错误（panic等）
    #[error("系统错误: {operation} - {message}")]
    System { operation: String, message: String },

    /// 超时错误
    #[error("超时错误: {operation} (超时: {timeout_ms}ms) - {message}")]
    Timeout {
        operation: String,
        timeout_ms: u64,
        message: String,
    },

    /// 资源不足错误
    #[error("资源不足: {resource} - {message}")]
    ResourceExhausted { resource: String, message: String },

    /// 未找到错误
    #[error("未找到: {resource} (ID: {id}) - {message}")]
    NotFound {
        resource: String,
        id: String,
        message: String,
    },

    /// 已存在错误
    #[error("已存在: {resource} (ID: {id}) - {message}")]
    AlreadyExists {
        resource: String,
        id: String,
        message: String,
    },

    /// 权限错误
    #[error("权限错误: {operation} - {message}")]
    Permission { operation: String, message: String },

    /// 网络错误
    #[error("网络错误: {operation} - {message}")]
    Network { operation: String, message: String },

    /// 序列化/反序列化错误
    #[error("序列化错误: {operation} - {message}")]
    Serialization { operation: String, message: String },

    /// 未知错误
    #[error("未知错误: {message}")]
    Unknown { message: String },
}

impl TerminalError {
    /// 创建配置错误
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// 创建面板错误
    pub fn pane<S1: Into<String>, S2: Into<String>>(
        operation: S1,
        pane_id: u32,
        message: S2,
    ) -> Self {
        Self::Pane {
            operation: operation.into(),
            pane_id,
            message: message.into(),
        }
    }

    /// 创建Shell错误
    pub fn shell<S1: Into<String>, S2: Into<String>>(operation: S1, message: S2) -> Self {
        Self::Shell {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// 创建I/O错误
    pub fn io<S1: Into<String>, S2: Into<String>>(operation: S1, message: S2) -> Self {
        Self::Io {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// 创建并发错误
    pub fn concurrency<S1: Into<String>, S2: Into<String>>(operation: S1, message: S2) -> Self {
        Self::Concurrency {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// 创建验证错误
    pub fn validation<S1: Into<String>, S2: Into<String>>(field: S1, message: S2) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// 创建系统错误
    pub fn system<S1: Into<String>, S2: Into<String>>(operation: S1, message: S2) -> Self {
        Self::System {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout<S1: Into<String>, S2: Into<String>>(
        operation: S1,
        timeout_ms: u64,
        message: S2,
    ) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout_ms,
            message: message.into(),
        }
    }

    /// 创建未找到错误
    pub fn not_found<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        resource: S1,
        id: S2,
        message: S3,
    ) -> Self {
        Self::NotFound {
            resource: resource.into(),
            id: id.into(),
            message: message.into(),
        }
    }

    /// 获取错误的严重程度
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Config { .. } => ErrorSeverity::High,
            Self::Pane { .. } => ErrorSeverity::Medium,
            Self::Shell { .. } => ErrorSeverity::Medium,
            Self::Io { .. } => ErrorSeverity::Medium,
            Self::Concurrency { .. } => ErrorSeverity::High,
            Self::Validation { .. } => ErrorSeverity::Low,
            Self::System { .. } => ErrorSeverity::Critical,
            Self::Timeout { .. } => ErrorSeverity::Medium,
            Self::ResourceExhausted { .. } => ErrorSeverity::High,
            Self::NotFound { .. } => ErrorSeverity::Low,
            Self::AlreadyExists { .. } => ErrorSeverity::Low,
            Self::Permission { .. } => ErrorSeverity::High,
            Self::Network { .. } => ErrorSeverity::Medium,
            Self::Serialization { .. } => ErrorSeverity::Medium,
            Self::Unknown { .. } => ErrorSeverity::Medium,
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config { .. } => "config",
            Self::Pane { .. } => "pane",
            Self::Shell { .. } => "shell",
            Self::Io { .. } => "io",
            Self::Concurrency { .. } => "concurrency",
            Self::Validation { .. } => "validation",
            Self::System { .. } => "system",
            Self::Timeout { .. } => "timeout",
            Self::ResourceExhausted { .. } => "resource",
            Self::NotFound { .. } => "not_found",
            Self::AlreadyExists { .. } => "already_exists",
            Self::Permission { .. } => "permission",
            Self::Network { .. } => "network",
            Self::Serialization { .. } => "serialization",
            Self::Unknown { .. } => "unknown",
        }
    }

    /// 是否可以重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Io { .. }
                | Self::Network { .. }
                | Self::Timeout { .. }
                | Self::Concurrency { .. }
                | Self::ResourceExhausted { .. }
        )
    }

    /// 记录错误日志
    pub fn log(&self) {
        match self.severity() {
            ErrorSeverity::Critical => error!("CRITICAL: {}", self),
            ErrorSeverity::High => error!("HIGH: {}", self),
            ErrorSeverity::Medium => error!("MEDIUM: {}", self),
            ErrorSeverity::Low => error!("LOW: {}", self),
        }
    }
}

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 结果类型别名
pub type TerminalResult<T> = Result<T, TerminalError>;

/// 错误处理工具
pub struct ErrorHandler;

impl ErrorHandler {
    /// 处理panic并转换为TerminalError
    pub fn handle_panic<T, F>(operation: &str, func: F) -> TerminalResult<T>
    where
        F: FnOnce() -> T + std::panic::UnwindSafe,
    {
        match std::panic::catch_unwind(func) {
            Ok(result) => Ok(result),
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "未知panic".to_string()
                };

                let error = TerminalError::system(operation, format!("panic: {}", panic_msg));
                error.log();
                Err(error)
            }
        }
    }

    /// 处理锁中毒错误
    pub fn handle_poison_error<T>(
        operation: &str,
        result: Result<T, std::sync::PoisonError<T>>,
    ) -> TerminalResult<T> {
        match result {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                let error = TerminalError::concurrency(
                    operation,
                    "锁被中毒，尝试恢复数据",
                );
                error.log();
                Ok(poisoned.into_inner())
            }
        }
    }

    /// 转换为Tauri命令错误格式
    pub fn to_tauri_error(error: TerminalError) -> String {
        error.log();
        error.to_string()
    }
}

/// 从标准错误类型转换
impl From<std::io::Error> for TerminalError {
    fn from(err: std::io::Error) -> Self {
        Self::io("I/O操作", err.to_string())
    }
}

impl From<serde_json::Error> for TerminalError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            operation: "JSON序列化".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<toml::de::Error> for TerminalError {
    fn from(err: toml::de::Error) -> Self {
        Self::Serialization {
            operation: "TOML反序列化".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<toml::ser::Error> for TerminalError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Serialization {
            operation: "TOML序列化".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for TerminalError {
    fn from(err: anyhow::Error) -> Self {
        Self::Unknown {
            message: err.to_string(),
        }
    }
}
