use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::agent::config::TaskExecutionConfig;
use crate::agent::context::FileContextTracker;
use crate::agent::persistence::AgentPersistence;
use crate::agent::ui::AgentUiPersistence;
use crate::storage::DatabaseManager;

#[derive(Debug, Clone)]
pub struct CompressedMemory {
    pub created_at: DateTime<Utc>,
    pub iteration_range: (u32, u32),
    pub summary: String,
    pub files_touched: Vec<String>,
    pub tools_used: Vec<String>,
    pub tokens_saved: u32,
}

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
    pub session_id: String,
    pub conversation_id: i64,
    pub workspace: PathBuf,
    pub initial_request: String,
    pub created_at: DateTime<Utc>,
    pub config: TaskExecutionConfig,

    compressed_history: Arc<RwLock<Vec<CompressedMemory>>>,
    file_tracker: Arc<FileContextTracker>,
    repositories: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
    ui_persistence: Arc<AgentUiPersistence>,
    stats: Arc<RwLock<SessionStats>>,
}

impl SessionContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        conversation_id: i64,
        workspace: PathBuf,
        initial_request: String,
        config: TaskExecutionConfig,
        repositories: Arc<DatabaseManager>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
    ) -> Self {
        let tracker = Arc::new(
            FileContextTracker::new(Arc::clone(&agent_persistence), conversation_id)
                .with_workspace_root(workspace.clone()),
        );

        Self {
            session_id,
            conversation_id,
            workspace,
            initial_request,
            created_at: Utc::now(),
            config,
            compressed_history: Arc::new(RwLock::new(Vec::new())),
            file_tracker: tracker,
            repositories,
            agent_persistence,
            ui_persistence,
            stats: Arc::new(RwLock::new(SessionStats::default())),
        }
    }

    pub fn repositories(&self) -> Arc<DatabaseManager> {
        Arc::clone(&self.repositories)
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.agent_persistence)
    }

    pub fn ui_persistence(&self) -> Arc<AgentUiPersistence> {
        Arc::clone(&self.ui_persistence)
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        Arc::clone(&self.file_tracker)
    }

    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    pub async fn add_compressed_memory(&self, memory: CompressedMemory) {
        let mut history = self.compressed_history.write().await;
        history.push(memory);
        if history.len() > 32 {
            let excess = history.len() - 32;
            history.drain(0..excess);
        }
    }

    pub async fn compressed_history(&self) -> Vec<CompressedMemory> {
        self.compressed_history.read().await.clone()
    }

    pub async fn get_compressed_history_text(&self) -> String {
        let history = self.compressed_history.read().await;
        if history.is_empty() {
            return String::new();
        }

        let mut text = String::from("## Compressed Session History\n\n");
        for memory in history.iter() {
            text.push_str(&format!(
                "**Iterations {}-{}** ({})\n{}\n",
                memory.iteration_range.0,
                memory.iteration_range.1,
                memory.created_at.format("%Y-%m-%d %H:%M:%S"),
                memory.summary
            ));
            if !memory.files_touched.is_empty() {
                text.push_str(&format!("_Files:_ {}\n", memory.files_touched.join(", ")));
            }
            if !memory.tools_used.is_empty() {
                text.push_str(&format!("_Tools:_ {}\n", memory.tools_used.join(", ")));
            }
            text.push('\n');
        }

        text
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
