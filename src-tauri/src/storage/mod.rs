/*!
 * 统一存储系统模块
 *
 * 实现三层存储架构：TOML配置层、MessagePack状态层、SQLite数据层
 * 提供统一的存储管理接口和协调器，支持缓存、错误恢复和数据迁移
 */

pub mod cache;
pub mod commands;
pub mod coordinator;
pub mod filesystem;
pub mod messagepack;
pub mod paths;
pub mod recovery;
pub mod sql_scripts;
pub mod sqlite;
#[cfg(test)]
pub mod sqlite_tests;
pub mod types;

// 重新导出核心类型和函数

pub use commands::StorageCoordinatorState;
pub use coordinator::{StorageCoordinator, StorageCoordinatorOptions};
pub use filesystem::{FileSystemManager, FileSystemOptions};
pub use messagepack::{MessagePackManager, MessagePackOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};
pub use recovery::{HealthCheckResult, RecoveryManager, RecoveryResult, SystemHealth};
pub use sqlite::{SqliteManager, SqliteOptions};
pub use types::{
    CacheLayer, CacheStats, DataQuery, SaveOptions, SessionState, StorageLayer, StorageStats,
};

// 统一错误处理类型
pub use crate::utils::error::{AppError, AppResult};

/// 存储系统版本
pub const STORAGE_VERSION: &str = "1.0.0";

/// 存储目录名称
pub const STORAGE_DIR_NAME: &str = "storage";
pub const CONFIG_DIR_NAME: &str = "config";
pub const STATE_DIR_NAME: &str = "state";
pub const DATA_DIR_NAME: &str = "data";

pub const BACKUPS_DIR_NAME: &str = "backups";

/// 文件名称
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const SESSION_STATE_FILE_NAME: &str = "session_state.msgpack";
pub const DATABASE_FILE_NAME: &str = "orbitx.db";
