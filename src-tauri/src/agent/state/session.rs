use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::agent::config::TaskExecutionConfig;
use crate::agent::context::FileContextTracker;
use crate::agent::persistence::AgentPersistence;
use crate::storage::DatabaseManager;

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_iterations: u32,
    pub total_tool_calls: u32,
    pub total_tokens_used: u64,
    pub total_cost: f64,
    pub files_read: u32,
    pub files_modified: u32,
}

pub struct SessionContext {
    pub task_id: String,
    pub session_id: i64,
    pub workspace: PathBuf,
    pub initial_request: String,
    pub created_at: DateTime<Utc>,
    pub config: TaskExecutionConfig,

    file_tracker: Arc<FileContextTracker>,
    repositories: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
    stats: Arc<RwLock<SessionStats>>,
}

impl SessionContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_id: String,
        session_id: i64,
        workspace: PathBuf,
        initial_request: String,
        config: TaskExecutionConfig,
        repositories: Arc<DatabaseManager>,
        agent_persistence: Arc<AgentPersistence>,
    ) -> Self {
        let tracker = Arc::new(
            FileContextTracker::new(
                Arc::clone(&agent_persistence),
                workspace.to_string_lossy().to_string(),
            )
            .with_workspace_root(workspace.clone()),
        );

        Self {
            task_id,
            session_id,
            workspace,
            initial_request,
            created_at: Utc::now(),
            config,
            file_tracker: tracker,
            repositories,
            agent_persistence,
            stats: Arc::new(RwLock::new(SessionStats::default())),
        }
    }

    pub fn repositories(&self) -> Arc<DatabaseManager> {
        Arc::clone(&self.repositories)
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.agent_persistence)
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        Arc::clone(&self.file_tracker)
    }

    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    pub async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut SessionStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut stats);
    }

    pub async fn stats(&self) -> SessionStats {
        self.stats.read().await.clone()
    }
}
