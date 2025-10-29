/*!
 * 存储系统模块
 *
 * 职责：
 * - database: SQLite 数据库管理
 * - cache: 统一内存缓存（带命名空间）
 * - messagepack: MessagePack 序列化存储
 * - repositories: 数据访问层（每个表一个结构体）
 * - paths: 路径管理
 * - error: 统一错误类型
 */

pub mod cache;
pub mod database;
pub mod error;
pub mod messagepack;
pub mod paths;
pub mod repositories;
pub mod sql_scripts;
pub mod types;

// ==================== 核心管理器 ====================
pub use cache::{CacheNamespace, UnifiedCache};
pub use database::{DatabaseManager, DatabaseOptions};
pub use messagepack::{MessagePackManager, MessagePackOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};

// ==================== 错误类型 ====================
pub use error::{
    CacheError, CacheResult, DatabaseError, DatabaseResult, MessagePackError, MessagePackResult,
    RepositoryError, RepositoryResult, SqlScriptError, SqlScriptResult, StorageError,
    StoragePathsError, StoragePathsResult, StorageResult,
};

// ==================== 通用类型 ====================
pub use types::{SessionState, StorageLayer};
// 存储系统版本
pub const STORAGE_VERSION: &str = "1.0.0";

// 存储目录名称
pub const STORAGE_DIR_NAME: &str = "storage";
pub const CONFIG_DIR_NAME: &str = "config";
pub const STATE_DIR_NAME: &str = "state";
pub const DATA_DIR_NAME: &str = "data";
pub const BACKUPS_DIR_NAME: &str = "backups";

// 文件名称
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const SESSION_STATE_FILE_NAME: &str = "session_state.msgpack";
pub const DATABASE_FILE_NAME: &str = "orbitx.db";
