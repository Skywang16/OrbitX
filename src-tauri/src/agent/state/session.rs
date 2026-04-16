use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::agent::config::AgentRunConfig;
use crate::agent::context::FileContextTracker;
use crate::agent::persistence::AgentPersistence;
use crate::storage::DatabaseManager;

#[derive(Debug, Clone, Default)]
pub struct ThreadStats {
    pub total_iterations: u32,
    pub total_tool_calls: u32,
    pub total_tokens_used: u64,
    pub total_cost: f64,
    pub files_read: u32,
    pub files_modified: u32,
}

pub struct ThreadContext {
    pub run_id: String,
    pub thread_id: i64,
    pub workspace: PathBuf,
    pub initial_request: String,
    pub created_at: DateTime<Utc>,
    pub config: AgentRunConfig,

    file_tracker: Arc<FileContextTracker>,
    repositories: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
    stats: Arc<RwLock<ThreadStats>>,
}

impl ThreadContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: String,
        thread_id: i64,
        workspace: PathBuf,
        initial_request: String,
        config: AgentRunConfig,
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
            run_id,
            thread_id,
            workspace,
            initial_request,
            created_at: Utc::now(),
            config,
            file_tracker: tracker,
            repositories,
            agent_persistence,
            stats: Arc::new(RwLock::new(ThreadStats::default())),
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

    pub fn config(&self) -> &AgentRunConfig {
        &self.config
    }

    pub async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut ThreadStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut stats);
    }

    pub async fn stats(&self) -> ThreadStats {
        self.stats.read().await.clone()
    }
}
