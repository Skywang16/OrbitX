use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tokio::sync::RwLock;

use crate::agent::persistence::{
    AgentPersistence, FileContextEntry, FileRecordSource, FileRecordState,
};
use crate::utils::error::AppResult;

const DEFAULT_RETENTION_DAYS: i64 = 14;

#[derive(Debug, Clone)]
pub struct FileOperationRecord<'a> {
    pub path: &'a Path,
    pub source: FileRecordSource,
    pub state_override: Option<FileRecordState>,
    pub recorded_at: DateTime<Utc>,
}

impl<'a> FileOperationRecord<'a> {
    pub fn new(path: &'a Path, source: FileRecordSource) -> Self {
        Self {
            path,
            source,
            state_override: None,
            recorded_at: Utc::now(),
        }
    }

    pub fn with_state(mut self, state: FileRecordState) -> Self {
        self.state_override = Some(state);
        self
    }

    pub fn recorded_at(mut self, recorded_at: DateTime<Utc>) -> Self {
        self.recorded_at = recorded_at;
        self
    }
}

#[derive(Debug)]
pub struct FileContextTracker {
    persistence: Arc<AgentPersistence>,
    conversation_id: i64,
    workspace_root: Option<PathBuf>,
    recently_modified: RwLock<HashSet<String>>, // user edits that require refresh
    recently_agent_edits: RwLock<HashSet<String>>, // agent changes to suppress stale warnings
    retention: Duration,
}

impl FileContextTracker {
    pub fn new(persistence: Arc<AgentPersistence>, conversation_id: i64) -> Self {
        let retention = ChronoDuration::days(DEFAULT_RETENTION_DAYS)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(14 * 24 * 60 * 60));

        Self {
            persistence,
            conversation_id,
            workspace_root: None,
            recently_modified: RwLock::new(HashSet::new()),
            recently_agent_edits: RwLock::new(HashSet::new()),
            retention,
        }
    }

    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn with_retention(mut self, retention: Duration) -> Self {
        self.retention = retention;
        self
    }

    pub async fn track_file_operation(
        &self,
        record: FileOperationRecord<'_>,
    ) -> AppResult<FileContextEntry> {
        let normalized_path = self.normalized_path(record.path);
        let repo = self.persistence.file_context();
        let existing = repo
            .find_by_path(self.conversation_id, &normalized_path)
            .await?;

        let now = record.recorded_at;
        let mut agent_read_ts = existing
            .as_ref()
            .and_then(|entry| entry.agent_read_timestamp);
        let mut agent_edit_ts = existing
            .as_ref()
            .and_then(|entry| entry.agent_edit_timestamp);
        let mut user_edit_ts = existing
            .as_ref()
            .and_then(|entry| entry.user_edit_timestamp);

        let mut mark_user_modified = false;
        let mut mark_agent_edit = false;
        let mut clear_user_modified = false;

        let mut state = match record.source {
            FileRecordSource::ReadTool => {
                agent_read_ts = Some(now);
                clear_user_modified = true;
                FileRecordState::Active
            }
            FileRecordSource::AgentEdited => {
                agent_read_ts = Some(now);
                agent_edit_ts = Some(now);
                mark_agent_edit = true;
                clear_user_modified = true;
                FileRecordState::Active
            }
            FileRecordSource::UserEdited => {
                user_edit_ts = Some(now);
                mark_user_modified = true;
                FileRecordState::Stale
            }
            FileRecordSource::FileMentioned => state_or(existing.as_ref(), FileRecordState::Active),
        };

        if let Some(override_state) = record.state_override {
            state = override_state;
        }

        let entry = repo
            .upsert_entry(
                self.conversation_id,
                &normalized_path,
                state,
                record.source,
                agent_read_ts,
                agent_edit_ts,
                user_edit_ts,
            )
            .await?;

        self.persistence
            .conversations()
            .touch(self.conversation_id)
            .await?;

        if mark_user_modified {
            let mut guard = self.recently_modified.write().await;
            guard.insert(normalized_path.clone());
        }

        if mark_agent_edit {
            let mut guard = self.recently_agent_edits.write().await;
            guard.insert(normalized_path.clone());
        }

        if clear_user_modified {
            let mut guard = self.recently_modified.write().await;
            guard.remove(&normalized_path);
        }

        Ok(entry)
    }

    pub async fn get_active_files(&self) -> AppResult<Vec<FileContextEntry>> {
        self.persistence
            .file_context()
            .get_active_files(self.conversation_id)
            .await
    }

    pub async fn get_stale_files(&self) -> AppResult<Vec<FileContextEntry>> {
        self.persistence
            .file_context()
            .get_stale_files(self.conversation_id)
            .await
    }

    pub async fn mark_file_as_stale(&self, path: impl AsRef<Path>) -> AppResult<FileContextEntry> {
        let record = FileOperationRecord::new(path.as_ref(), FileRecordSource::UserEdited)
            .with_state(FileRecordState::Stale);
        self.track_file_operation(record).await
    }

    pub async fn cleanup_old_entries(&self) -> AppResult<u64> {
        let repo = self.persistence.file_context();
        let cutoff_delta = ChronoDuration::from_std(self.retention)
            .unwrap_or_else(|_| ChronoDuration::days(DEFAULT_RETENTION_DAYS));
        let cutoff = Utc::now()
            .checked_sub_signed(cutoff_delta)
            .unwrap_or_else(Utc::now);

        repo.delete_stale_entries_before(self.conversation_id, cutoff)
            .await
    }

    pub async fn take_recently_modified(&self) -> Vec<String> {
        let mut guard = self.recently_modified.write().await;
        guard.drain().collect()
    }

    pub async fn take_recent_agent_edits(&self) -> Vec<String> {
        let mut guard = self.recently_agent_edits.write().await;
        guard.drain().collect()
    }

    fn derive_state(&self, source: &FileRecordSource) -> FileRecordState {
        match source {
            FileRecordSource::UserEdited => FileRecordState::Stale,
            _ => FileRecordState::Active,
        }
    }

    fn normalized_path(&self, path: &Path) -> String {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(root) = &self.workspace_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        resolved
            .components()
            .collect::<PathBuf>()
            .to_string_lossy()
            .replace('\\', "/")
    }
}

fn state_or(existing: Option<&FileContextEntry>, fallback: FileRecordState) -> FileRecordState {
    existing.map(|entry| entry.record_state).unwrap_or(fallback)
}
