/*!
 * 统一存储系统模块
 *
 * 实现三层存储架构：TOML配置层、MessagePack状态层、SQLite数据层
 * 采用Repository模式和查询构建器，提供类型安全的数据访问接口
 */

// 核心模块
pub mod cache;
pub mod coordinator;
// pub mod filesystem; // 已移动到 ai::tool::filesystem
pub mod messagepack;
pub mod paths;
pub mod recovery;
pub mod types;

// 数据库相关模块
pub mod database;
pub mod query;
pub mod repositories;

// 命令和脚本
pub mod sql_scripts;

// 重新导出核心类型和功能
pub use cache::UnifiedCache;
pub use coordinator::{StorageCoordinator, StorageCoordinatorOptions};
pub use database::{DatabaseManager, DatabaseOptions};
// pub use filesystem::{FileSystemManager, FileSystemOptions}; // 已移动到 ai::tool::filesystem
pub use messagepack::{MessagePackManager, MessagePackOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};
pub use query::{QueryCondition, QueryOrder, SafeQueryBuilder};
pub use repositories::*;
pub use types::{SessionState, StorageLayer};

// 重新导出统一的错误处理类型
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
