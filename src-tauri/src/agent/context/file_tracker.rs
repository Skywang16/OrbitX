use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::error::AgentResult;
use crate::agent::persistence::{
    AgentPersistence, FileRecordSource, FileRecordState, WorkspaceFileRecord,
};

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
    workspace_path: String,
    workspace_root: Option<PathBuf>,
    recently_modified: RwLock<HashSet<String>>, // user edits that require refresh
    recently_agent_edits: RwLock<HashSet<String>>, // agent changes to suppress stale warnings
}

impl FileContextTracker {
    pub fn new(persistence: Arc<AgentPersistence>, workspace_path: impl Into<String>) -> Self {
        Self {
            persistence,
            workspace_path: workspace_path.into(),
            workspace_root: None,
            recently_modified: RwLock::new(HashSet::new()),
            recently_agent_edits: RwLock::new(HashSet::new()),
        }
    }

    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn normalize_path(&self, path: impl AsRef<Path>) -> String {
        self.normalized_path(path.as_ref())
    }

    pub async fn track_file_operation(
        &self,
        record: FileOperationRecord<'_>,
    ) -> AgentResult<WorkspaceFileRecord> {
        let normalized_path = self.normalized_path(record.path);
        let repo = self.persistence.file_context();
        let existing = repo
            .find_by_path(&self.workspace_path, &normalized_path)
            .await?;

        let now = record.recorded_at;
        let mut agent_read_ts = existing
            .as_ref()
            .and_then(|entry| entry.agent_read_at);
        let mut agent_edit_ts = existing
            .as_ref()
            .and_then(|entry| entry.agent_edit_at);
        let mut user_edit_ts = existing
            .as_ref()
            .and_then(|entry| entry.user_edit_at);

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
            FileRecordSource::FileMentioned => {
                state_or(existing.as_ref(), FileRecordState::Active)
            }
        };

        if let Some(override_state) = record.state_override {
            state = override_state;
        }

        let entry = repo
            .upsert_entry(
                &self.workspace_path,
                &normalized_path,
                state,
                record.source,
                agent_read_ts,
                agent_edit_ts,
                user_edit_ts,
            )
            .await?;

        self.persistence
            .workspaces()
            .touch(&self.workspace_path)
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

    pub async fn get_active_files(&self) -> AgentResult<Vec<WorkspaceFileRecord>> {
        self.persistence
            .file_context()
            .list_by_state(&self.workspace_path, FileRecordState::Active)
            .await
    }

    pub async fn get_stale_files(&self) -> AgentResult<Vec<WorkspaceFileRecord>> {
        self.persistence
            .file_context()
            .list_by_state(&self.workspace_path, FileRecordState::Stale)
            .await
    }

    pub async fn mark_file_as_stale(
        &self,
        path: impl AsRef<Path>,
    ) -> AgentResult<WorkspaceFileRecord> {
        let record = FileOperationRecord::new(path.as_ref(), FileRecordSource::UserEdited)
            .with_state(FileRecordState::Stale);
        self.track_file_operation(record).await
    }

    pub async fn take_recently_modified(&self) -> Vec<String> {
        let mut guard = self.recently_modified.write().await;
        guard.drain().collect()
    }

    pub async fn take_recent_agent_edits(&self) -> Vec<String> {
        let mut guard = self.recently_agent_edits.write().await;
        guard.drain().collect()
    }

    fn normalized_path(&self, path: &Path) -> String {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(root) = &self.workspace_root {
            root.join(path)
        } else {
            PathBuf::from(&self.workspace_path).join(path)
        };

        let workspace_root = self
            .workspace_root
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.workspace_path));

        let relative = resolved
            .strip_prefix(&workspace_root)
            .map(|p| p.to_path_buf())
            .unwrap_or(resolved);

        relative
            .components()
            .collect::<PathBuf>()
            .to_string_lossy()
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .replace('\\', "/")
    }
}

fn state_or(existing: Option<&WorkspaceFileRecord>, fallback: FileRecordState) -> FileRecordState {
    existing.map(|entry| entry.record_state).unwrap_or(fallback)
}
