/*!
 * 统一存储系统模块
 *
 * 实现三层存储架构：TOML配置层、MessagePack状态层、SQLite数据层
 * 提供统一的存储管理接口和协调器，支持缓存、错误恢复和数据迁移
 */

pub mod coordinator;
pub mod error;
pub mod filesystem;
pub mod messagepack;
pub mod paths;
pub mod sqlite;
pub mod toml_config;
pub mod types;

// 重新导出核心类型和函数
pub use coordinator::{StorageCoordinator, StorageCoordinatorOptions};
pub use error::{StorageError, StorageResult};
pub use filesystem::{FileSystemManager, FileSystemOptions};
pub use messagepack::{MessagePackManager, MessagePackOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};
pub use sqlite::{SqliteManager, SqliteOptions};
pub use toml_config::{TomlConfigManager, TomlConfigOptions};
pub use types::{
    CacheLayer, CacheStats, DataQuery, SaveOptions, SessionState, StorageLayer, StorageStats,
};

/// 存储系统版本
pub const STORAGE_VERSION: &str = "1.0.0";

/// 存储目录名称
pub const STORAGE_DIR_NAME: &str = "storage";
pub const CONFIG_DIR_NAME: &str = "config";
pub const STATE_DIR_NAME: &str = "state";
pub const DATA_DIR_NAME: &str = "data";
pub const CACHE_DIR_NAME: &str = "cache";
pub const BACKUPS_DIR_NAME: &str = "backups";

/// 文件名称
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const SESSION_STATE_FILE_NAME: &str = "session_state.msgpack";
pub const DATABASE_FILE_NAME: &str = "termx.db";

/// 缓存配置
pub const DEFAULT_CACHE_SIZE: usize = 1024 * 1024; // 1MB
pub const DEFAULT_LRU_CAPACITY: usize = 1000;
pub const DEFAULT_TTL_SECONDS: u64 = 3600; // 1小时
