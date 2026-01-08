//! Checkpoint 数据模型

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Checkpoint 错误类型
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspace(String),

    #[error("File path outside workspace: {0}")]
    InvalidFilePath(String),

    #[error("Checkpoint not found: {0}")]
    NotFound(i64),

    #[error("Blob not found: {0}")]
    BlobNotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type CheckpointResult<T> = Result<T, CheckpointError>;

fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_default()
}

/// Checkpoint 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub id: i64,
    pub workspace_path: String,
    pub session_id: i64,
    pub message_id: i64,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> CheckpointResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            workspace_path: row.try_get("workspace_path")?,
            session_id: row.try_get("session_id")?,
            message_id: row.try_get("message_id")?,
            parent_id: row.try_get("parent_id")?,
            created_at: timestamp_to_datetime(row.try_get("created_at")?),
        })
    }
}

/// Checkpoint 摘要（包含文件统计）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSummary {
    pub id: i64,
    pub workspace_path: String,
    pub session_id: i64,
    pub message_id: i64,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub file_count: i64,
    pub total_size: i64,
}

impl CheckpointSummary {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> CheckpointResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            workspace_path: row.try_get("workspace_path")?,
            session_id: row.try_get("session_id")?,
            message_id: row.try_get("message_id")?,
            parent_id: row.try_get("parent_id")?,
            created_at: timestamp_to_datetime(row.try_get("created_at")?),
            file_count: row.try_get("file_count")?,
            total_size: row.try_get("total_size")?,
        })
    }
}

/// 文件变更类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
}

impl FileChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
        }
    }
}

impl FromStr for FileChangeType {
    type Err = CheckpointError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "added" => Ok(Self::Added),
            "modified" => Ok(Self::Modified),
            "deleted" => Ok(Self::Deleted),
            other => Err(CheckpointError::Parse(format!(
                "Unknown file change type: {}",
                other
            ))),
        }
    }
}

/// 文件快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub id: i64,
    pub checkpoint_id: i64,
    pub file_path: String,
    pub blob_hash: String,
    pub change_type: FileChangeType,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

impl FileSnapshot {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> CheckpointResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            checkpoint_id: row.try_get("checkpoint_id")?,
            file_path: row.try_get("file_path")?,
            blob_hash: row.try_get("blob_hash")?,
            change_type: row.try_get::<String, _>("change_type")?.parse()?,
            file_size: row.try_get("file_size")?,
            created_at: timestamp_to_datetime(row.try_get("created_at")?),
        })
    }
}

/// 文件差异
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub file_path: String,
    pub change_type: FileChangeType,
    pub diff_content: Option<String>,
}

/// 回滚结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    pub checkpoint_id: i64,
    pub restored_files: Vec<String>,
    pub failed_files: Vec<(String, String)>,
}

/// 创建 Checkpoint 的参数
#[derive(Debug, Clone)]
pub struct NewCheckpoint {
    pub workspace_path: String,
    pub session_id: i64,
    pub message_id: i64,
    pub parent_id: Option<i64>,
}

/// 创建文件快照的参数
#[derive(Debug, Clone)]
pub struct NewFileSnapshot {
    pub checkpoint_id: i64,
    pub file_path: String,
    pub blob_hash: String,
    pub change_type: FileChangeType,
    pub file_size: i64,
}
