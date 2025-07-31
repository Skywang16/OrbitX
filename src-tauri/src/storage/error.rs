/*!
 * 存储系统错误处理模块
 *
 * 定义存储系统专用的错误类型和错误处理机制，
 * 提供统一的错误恢复策略和错误上下文信息
 */

use crate::utils::error::{AppError, AppResult};
use std::path::PathBuf;
use thiserror::Error;

/// 存储系统专用结果类型
pub type StorageResult<T> = Result<T, StorageError>;

/// 存储系统错误类型
#[derive(Debug, Error)]
pub enum StorageError {
    /// 配置文件相关错误
    #[error("配置文件错误: {message}")]
    ConfigError {
        message: String,
        path: Option<PathBuf>,
    },

    /// 会话状态相关错误
    #[error("会话状态错误: {message}")]
    SessionStateError {
        message: String,
        path: Option<PathBuf>,
    },

    /// 数据库相关错误
    #[error("数据库错误: {message}")]
    DatabaseError {
        message: String,
        path: Option<PathBuf>,
    },

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// 反序列化错误
    #[error("反序列化错误: {0}")]
    DeserializationError(String),

    /// 加密相关错误
    #[error("加密错误: {0}")]
    EncryptionError(String),

    /// 文件系统IO错误
    #[error("文件系统错误: {message}")]
    FileSystemError {
        message: String,
        path: Option<PathBuf>,
    },

    /// 缓存相关错误
    #[error("缓存错误: {0}")]
    CacheError(String),

    /// 迁移相关错误
    #[error("数据迁移错误: {message}")]
    MigrationError {
        message: String,
        from_version: Option<String>,
        to_version: Option<String>,
    },

    /// 权限相关错误
    #[error("权限错误: {0}")]
    PermissionError(String),

    /// 数据验证错误
    #[error("数据验证错误: {0}")]
    ValidationError(String),

    /// 网络相关错误（用于远程存储）
    #[error("网络错误: {0}")]
    NetworkError(String),

    /// 通用错误
    #[error("存储系统错误: {0}")]
    Generic(String),
}

impl StorageError {
    /// 创建配置错误
    pub fn config_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::ConfigError {
            message: message.into(),
            path,
        }
    }

    /// 创建会话状态错误
    pub fn session_state_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::SessionStateError {
            message: message.into(),
            path,
        }
    }

    /// 创建数据库错误
    pub fn database_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::DatabaseError {
            message: message.into(),
            path,
        }
    }

    /// 创建文件系统错误
    pub fn filesystem_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::FileSystemError {
            message: message.into(),
            path,
        }
    }

    /// 创建迁移错误
    pub fn migration_error(
        message: impl Into<String>,
        from_version: Option<String>,
        to_version: Option<String>,
    ) -> Self {
        Self::MigrationError {
            message: message.into(),
            from_version,
            to_version,
        }
    }

    /// 获取错误相关的文件路径
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::ConfigError { path, .. }
            | Self::SessionStateError { path, .. }
            | Self::DatabaseError { path, .. }
            | Self::FileSystemError { path, .. } => path.as_ref(),
            _ => None,
        }
    }

    /// 检查是否为可恢复的错误
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ConfigError { .. }
                | Self::SessionStateError { .. }
                | Self::CacheError(_)
                | Self::NetworkError(_)
        )
    }

    /// 检查是否为致命错误
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::PermissionError(_) | Self::FileSystemError { .. }
        )
    }
}

// 实现从其他错误类型的转换
impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::FileSystemError {
            message: err.to_string(),
            path: None,
        }
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<toml::de::Error> for StorageError {
    fn from(err: toml::de::Error) -> Self {
        Self::DeserializationError(format!("TOML解析错误: {}", err))
    }
}

impl From<toml::ser::Error> for StorageError {
    fn from(err: toml::ser::Error) -> Self {
        Self::SerializationError(format!("TOML序列化错误: {}", err))
    }
}

// 转换为通用应用错误
impl From<StorageError> for AppError {
    fn from(err: StorageError) -> Self {
        AppError::new(err)
    }
}

/// 错误恢复策略
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 使用默认值
    UseDefault,
    /// 从备份恢复
    RestoreFromBackup,
    /// 重新创建
    Recreate,
    /// 降级模式
    Fallback,
    /// 用户干预
    UserIntervention,
}

/// 错误恢复管理器
pub struct ErrorRecoveryManager {
    backup_enabled: bool,
    fallback_enabled: bool,
}

impl ErrorRecoveryManager {
    pub fn new(backup_enabled: bool, fallback_enabled: bool) -> Self {
        Self {
            backup_enabled,
            fallback_enabled,
        }
    }

    /// 获取错误的推荐恢复策略
    pub fn get_recovery_strategy(&self, error: &StorageError) -> RecoveryStrategy {
        match error {
            StorageError::ConfigError { .. } => {
                if self.backup_enabled {
                    RecoveryStrategy::RestoreFromBackup
                } else {
                    RecoveryStrategy::UseDefault
                }
            }
            StorageError::SessionStateError { .. } => {
                if self.backup_enabled {
                    RecoveryStrategy::RestoreFromBackup
                } else {
                    RecoveryStrategy::UseDefault
                }
            }
            StorageError::DatabaseError { .. } => {
                if self.backup_enabled {
                    RecoveryStrategy::RestoreFromBackup
                } else {
                    RecoveryStrategy::Recreate
                }
            }
            StorageError::FileSystemError { .. } => RecoveryStrategy::UserIntervention,
            StorageError::PermissionError(_) => RecoveryStrategy::UserIntervention,
            StorageError::CacheError(_) => RecoveryStrategy::Recreate,
            _ => {
                if self.fallback_enabled {
                    RecoveryStrategy::Fallback
                } else {
                    RecoveryStrategy::UseDefault
                }
            }
        }
    }

    /// 执行错误恢复
    pub async fn recover(&self, error: &StorageError) -> StorageResult<()> {
        let strategy = self.get_recovery_strategy(error);

        match strategy {
            RecoveryStrategy::UseDefault => {
                log::info!("使用默认值恢复: {}", error);
                Ok(())
            }
            RecoveryStrategy::RestoreFromBackup => {
                log::info!("从备份恢复: {}", error);
                // TODO: 实现备份恢复逻辑
                Ok(())
            }
            RecoveryStrategy::Recreate => {
                log::info!("重新创建: {}", error);
                // TODO: 实现重新创建逻辑
                Ok(())
            }
            RecoveryStrategy::Fallback => {
                log::info!("降级模式: {}", error);
                // TODO: 实现降级逻辑
                Ok(())
            }
            RecoveryStrategy::UserIntervention => {
                log::error!("需要用户干预: {}", error);
                Err(StorageError::Generic("需要用户干预才能恢复".to_string()))
            }
        }
    }
}

/// 便捷的错误创建宏
#[macro_export]
macro_rules! storage_bail {
    ($err:expr) => {
        return Err($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(StorageError::Generic(format!($fmt, $($arg)*)))
    };
}

/// 便捷的错误创建函数
pub fn storage_error(message: impl Into<String>) -> StorageError {
    StorageError::Generic(message.into())
}

/// 将 StorageResult 转换为 AppResult
pub fn to_app_result<T>(result: StorageResult<T>) -> AppResult<T> {
    result.map_err(|e| e.into())
}

/// 为 StorageResult 提供便捷的转换方法
pub trait ToAppResult<T> {
    fn to_app(self) -> AppResult<T>;
}

impl<T> ToAppResult<T> for StorageResult<T> {
    fn to_app(self) -> AppResult<T> {
        to_app_result(self)
    }
}
