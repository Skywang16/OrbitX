/*!
 * Storage system module
 *
 * Responsibilities:
 * - database: SQLite database management
 * - cache: unified in-memory cache (with namespaces)
 * - repositories: data access layer (one struct per table)
 * - paths: path management
 * - error: unified error types
 */

pub mod cache;
pub mod database;
pub mod error;
pub mod paths;
pub mod repositories;
pub mod sql_scripts;
pub mod types;

// ==================== Core Managers ====================
pub use cache::{CacheNamespace, UnifiedCache};
pub use database::{DatabaseManager, DatabaseOptions};
pub use paths::{StoragePaths, StoragePathsBuilder};

// ==================== Error Types ====================
pub use error::{
    CacheError, CacheResult, DatabaseError, DatabaseResult, RepositoryError, RepositoryResult,
    SqlScriptError, SqlScriptResult, StorageError, StoragePathsError, StoragePathsResult,
    StorageResult,
};

// ==================== Common Types ====================
pub use types::StorageLayer;
// Storage system version
pub const STORAGE_VERSION: &str = "1.0.0";

// Storage directory names
pub const STORAGE_DIR_NAME: &str = "storage";
pub const CONFIG_DIR_NAME: &str = "config";
pub const STATE_DIR_NAME: &str = "state";
pub const DATA_DIR_NAME: &str = "data";
pub const BACKUPS_DIR_NAME: &str = "backups";

// File names
pub const DATABASE_FILE_NAME: &str = "orbitx.db";
