use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::types::{RolloutItem, RolloutLine, ThreadMeta};
use crate::storage::DatabaseManager;

static ROLLOUT_FILE_LOCKS: Lazy<DashMap<PathBuf, Arc<Mutex<()>>>> = Lazy::new(DashMap::new);

#[derive(Debug, Clone)]
pub struct RolloutRecorder {
    database: Arc<DatabaseManager>,
}

impl RolloutRecorder {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    pub fn threads_root(&self) -> PathBuf {
        self.database.state_dir().join("threads")
    }

    pub fn thread_dir(&self, thread_id: i64) -> PathBuf {
        self.threads_root().join(thread_id.to_string())
    }

    pub fn rollout_path(&self, thread_id: i64) -> PathBuf {
        self.thread_dir(thread_id).join("rollout.jsonl")
    }

    pub async fn ensure_thread_rollout(&self, meta: &ThreadMeta) -> io::Result<PathBuf> {
        let dir = self.thread_dir(meta.thread_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join("rollout.jsonl");
        if tokio::fs::try_exists(&path).await? {
            return Ok(path);
        }
        self.append_to_path(&path, RolloutItem::ThreadMeta(meta.clone()))
            .await?;
        Ok(path)
    }

    pub async fn append(&self, thread_id: i64, item: RolloutItem) -> io::Result<PathBuf> {
        let path = self.rollout_path(thread_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        self.append_to_path(&path, item).await?;
        Ok(path)
    }

    async fn append_to_path(&self, path: &Path, item: RolloutItem) -> io::Result<()> {
        let path_buf = path.to_path_buf();
        let lock = ROLLOUT_FILE_LOCKS
            .entry(path_buf.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_buf)
            .await?;
        let line = RolloutLine {
            timestamp: Utc::now(),
            item,
        };
        let encoded = serde_json::to_vec(&line)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        file.write_all(&encoded).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}
