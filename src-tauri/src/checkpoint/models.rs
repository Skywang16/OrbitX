//! Checkpoint 系统数据模型定义

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Checkpoint 相关错误
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("解析错误: {0}")]
    Parse(String),

    #[error("工作区路径无效: {0}")]
    InvalidWorkspace(String),

    #[error("文件路径不在工作区: {0}")]
    InvalidFilePath(String),

    #[error("Checkpoint 不存在: {0}")]
    NotFound(i64),

    #[error("Blob 不存在: {0}")]
    BlobNotFound(String),
}

pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// 时间戳转 DateTime
pub fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_default()
}

/// Checkpoint 记录：表示某个时间点的文件状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub id: i64,
    pub workspace_path: String,
    pub session_id: i64,
    pub parent_id: Option<i64>,
    pub user_message: String,
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    pub fn new(
        id: i64,
        workspace_path: String,
        session_id: i64,
        parent_id: Option<i64>,
        user_message: String,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            workspace_path,
            session_id,
            parent_id,
            user_message,
            created_at: timestamp_to_datetime(created_at),
        }
    }
}

/// Checkpoint 摘要：包含统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSummary {
    pub id: i64,
    pub workspace_path: String,
    pub session_id: i64,
    pub parent_id: Option<i64>,
    pub user_message: String,
    pub created_at: DateTime<Utc>,
    pub file_count: i64,
    pub total_size: i64,
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

/// 文件快照：记录某个文件在 checkpoint 时的状态
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
    pub fn new(
        id: i64,
        checkpoint_id: i64,
        file_path: String,
        blob_hash: String,
        change_type: FileChangeType,
        file_size: i64,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            checkpoint_id,
            file_path,
            blob_hash,
            change_type,
            file_size,
            created_at: timestamp_to_datetime(created_at),
        }
    }
}

/// 文件差异：两个状态之间的文件变化
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub file_path: String,
    pub change_type: FileChangeType,
    /// unified diff 格式的差异内容，删除的文件为 None
    pub diff_content: Option<String>,
}

/// 回滚结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    /// 回滚目标 checkpoint ID
    pub checkpoint_id: i64,
    /// 回滚后创建的新 checkpoint ID
    pub new_checkpoint_id: i64,
    /// 成功恢复的文件列表
    pub restored_files: Vec<String>,
    /// 恢复失败的文件列表：(文件路径, 错误信息)
    pub failed_files: Vec<(String, String)>,
}

/// 新建 Checkpoint 的输入参数
#[derive(Debug, Clone)]
pub struct NewCheckpoint {
    pub workspace_path: String,
    pub session_id: i64,
    pub parent_id: Option<i64>,
    pub user_message: String,
}

/// 新建文件快照的输入参数
#[derive(Debug, Clone)]
pub struct NewFileSnapshot {
    pub checkpoint_id: i64,
    pub file_path: String,
    pub blob_hash: String,
    pub change_type: FileChangeType,
    pub file_size: i64,
}
