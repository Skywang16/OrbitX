// 统一存储系统模块

pub mod error;
pub mod cache;
pub mod coordinator;
pub mod messagepack;
pub mod paths;
pub mod recovery;
pub mod types;

pub mod database;
pub mod query;
pub mod repositories;

pub mod sql_scripts;

pub use cache::UnifiedCache;
pub use coordinator::{StorageCoordinator, StorageCoordinatorOptions};
pub use database::{DatabaseManager, DatabaseOptions};
pub use messagepack::{MessagePackManager, MessagePackOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};
pub use query::{QueryCondition, QueryOrder, SafeQueryBuilder};
pub use repositories::*;
pub use types::{SessionState, StorageLayer};
pub use error::{
    CacheError, CacheResult, DatabaseError, DatabaseResult, MessagePackError, MessagePackResult,
    QueryBuilderError, QueryResult, RepositoryError, RepositoryResult, SqlScriptError,
    SqlScriptResult, StorageCoordinatorError, StorageCoordinatorResult, StorageError,
    StoragePathsError, StoragePathsResult, StorageRecoveryError, StorageRecoveryResult, StorageResult,
};
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
