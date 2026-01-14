use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

use crate::agent::error::AgentResult;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRecordState {
    Active,
    Stale,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRecordSource {
    ReadTool,
    UserEdited,
    AgentEdited,
    FileMentioned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFileRecord {
    pub relative_path: String,
    pub record_state: FileRecordState,
    pub recorded_at: DateTime<Utc>,
}

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
    workspace_path: String,
    workspace_root: Option<PathBuf>,

    active: RwLock<HashSet<String>>,
    stale: RwLock<HashSet<String>>,
    recently_modified: RwLock<HashSet<String>>,
    recently_agent_edits: RwLock<HashSet<String>>,
}

impl FileContextTracker {
    pub fn new(
        _persistence: std::sync::Arc<crate::agent::persistence::AgentPersistence>,
        workspace_path: impl Into<String>,
    ) -> Self {
        Self {
            workspace_path: workspace_path.into(),
            workspace_root: None,
            active: RwLock::new(HashSet::new()),
            stale: RwLock::new(HashSet::new()),
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
    ) -> AgentResult<TrackedFileRecord> {
        let normalized_path = self.normalized_path(record.path);
        let now = record.recorded_at;

        let state = record
            .state_override
            .unwrap_or_else(|| match record.source {
                FileRecordSource::ReadTool | FileRecordSource::AgentEdited => FileRecordState::Active,
                FileRecordSource::UserEdited => FileRecordState::Stale,
                FileRecordSource::FileMentioned => FileRecordState::Active,
            });

        match state {
            FileRecordState::Active => {
                self.active.write().await.insert(normalized_path.clone());
                self.stale.write().await.remove(&normalized_path);
                self.recently_modified.write().await.remove(&normalized_path);
            }
            FileRecordState::Stale => {
                self.stale.write().await.insert(normalized_path.clone());
                self.active.write().await.remove(&normalized_path);
                self.recently_modified.write().await.insert(normalized_path.clone());
            }
        }

        if matches!(record.source, FileRecordSource::AgentEdited) {
            self.recently_agent_edits
                .write()
                .await
                .insert(normalized_path.clone());
        }

        Ok(TrackedFileRecord {
            relative_path: normalized_path,
            record_state: state,
            recorded_at: now,
        })
    }

    pub async fn get_active_files(&self) -> AgentResult<Vec<TrackedFileRecord>> {
        let now = Utc::now();
        let guard = self.active.read().await;
        Ok(guard
            .iter()
            .cloned()
            .map(|relative_path| TrackedFileRecord {
                relative_path,
                record_state: FileRecordState::Active,
                recorded_at: now,
            })
            .collect())
    }

    pub async fn get_stale_files(&self) -> AgentResult<Vec<TrackedFileRecord>> {
        let now = Utc::now();
        let guard = self.stale.read().await;
        Ok(guard
            .iter()
            .cloned()
            .map(|relative_path| TrackedFileRecord {
                relative_path,
                record_state: FileRecordState::Stale,
                recorded_at: now,
            })
            .collect())
    }

    pub async fn mark_file_as_stale(
        &self,
        path: impl AsRef<Path>,
    ) -> AgentResult<TrackedFileRecord> {
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
