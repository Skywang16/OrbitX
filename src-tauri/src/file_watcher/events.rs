use serde::{Deserialize, Serialize};

pub fn now_timestamp_ms() -> u64 {
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeType {
    Index,
    Head,
    Refs,
    Worktree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWatcherEventBatch {
    pub seq: u64,
    pub events: Vec<FileWatcherEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileWatcherEvent {
    #[serde(rename_all = "camelCase")]
    #[serde(rename = "git_changed")]
    Git {
        repo_root: String,
        change_type: GitChangeType,
        timestamp_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    #[serde(rename = "fs_changed")]
    Fs {
        workspace_root: String,
        path: String,
        event_type: FsEventType,
        old_path: Option<String>,
        timestamp_ms: u64,
    },
}

impl FileWatcherEvent {
    pub fn git_changed(repo_root: String, change_type: GitChangeType) -> Self {
        Self::Git {
            repo_root,
            change_type,
            timestamp_ms: now_timestamp_ms(),
        }
    }

    pub fn fs_changed(
        workspace_root: String,
        path: String,
        event_type: FsEventType,
        old_path: Option<String>,
    ) -> Self {
        Self::Fs {
            workspace_root,
            path,
            event_type,
            old_path,
            timestamp_ms: now_timestamp_ms(),
        }
    }
}
