/*!
 * AgentRunExecutor - Agent run executor
 *
 * Responsibilities:
 * - Agent run lifecycle management (create, pause, resume, cancel)
 * - Agent run state query and management
 * - Coordinate PromptOrchestrator and ReactOrchestrator
 *
 * Non-responsibilities (already separated):
 * - Prompt building -> agent/prompt/orchestrator.rs
 * - ReAct loop -> agent/react/orchestrator/mod.rs
 * - Tool execution -> agent/tools/
 * - Persistence -> agent/persistence/
 */

mod builder;
mod lifecycle;
mod react_handler;
mod react_impl;
mod state;
mod subagent;
mod types;

pub use react_handler::ReactHandler;
pub use state::AgentRunExecutorStats;
pub use types::*;

use std::sync::Arc;

use dashmap::DashMap;

use crate::agent::mcp::McpRegistry;
use crate::agent::persistence::AgentPersistence;
use crate::agent::prompt::orchestrator::PromptOrchestrator;
use crate::agent::react::orchestrator::ReactOrchestrator;
use crate::agent::tools::ToolConfirmationManager;
use crate::agent::workspace_changes::WorkspaceChangeJournal;
use crate::checkpoint::CheckpointService;
use crate::lsp::LspManager;
use crate::settings::SettingsManager;
use crate::storage::{DatabaseManager, UnifiedCache};

/// AgentRunExecutor internal state
struct AgentRunExecutorInner {
    // Core services
    database: Arc<DatabaseManager>,
    cache: Arc<UnifiedCache>,
    agent_persistence: Arc<AgentPersistence>,
    settings_manager: Arc<SettingsManager>,
    mcp_registry: Arc<McpRegistry>,
    lsp_manager: Arc<LspManager>,

    // Checkpoint service (optional, for automatic checkpoint creation)
    checkpoint_service: Option<Arc<CheckpointService>>,
    workspace_changes: Arc<WorkspaceChangeJournal>,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    tool_confirmations: Arc<ToolConfirmationManager>,

    // Orchestrators
    prompt_orchestrator: Arc<PromptOrchestrator>,
    react_orchestrator: Arc<ReactOrchestrator>,

    // Active run state management - only used to find running runs for interruption
    // No longer cache conversation_contexts, load from DB each time
    active_runs: DashMap<String, Arc<crate::agent::core::context::AgentRunContext>>,
    active_child_executions_by_parent: DashMap<String, usize>,
}

pub(crate) struct AgentRunExecutorServices {
    pub database: Arc<DatabaseManager>,
    pub cache: Arc<UnifiedCache>,
    pub agent_persistence: Arc<AgentPersistence>,
    pub settings_manager: Arc<SettingsManager>,
    pub mcp_registry: Arc<McpRegistry>,
    pub lsp_manager: Arc<LspManager>,
    pub checkpoint_service: Option<Arc<CheckpointService>>,
    pub workspace_changes: Arc<WorkspaceChangeJournal>,
    pub vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
}

/// AgentRunExecutor - Agent run executor
///
/// - All APIs return Arc<AgentRunContext>, caller manages lifecycle
/// - DashMap directly stores Arc, only increments reference count when accessed
#[derive(Clone)]
pub struct AgentRunExecutor {
    inner: Arc<AgentRunExecutorInner>,
}

#[async_trait::async_trait]
impl crate::agent::core::context::SubAgentRunner for AgentRunExecutor {
    async fn run_subagent(
        &self,
        parent: &crate::agent::core::context::AgentRunContext,
        request: crate::agent::core::context::SubAgentRequest,
    ) -> crate::agent::error::AgentRunResult<crate::agent::core::context::SubAgentResponse> {
        subagent::run_subagent_task(self, parent, request).await
    }

    async fn spawn_subagent(
        &self,
        parent: &crate::agent::core::context::AgentRunContext,
        request: crate::agent::core::context::SubAgentRequest,
        collab_call_id: String,
        collab_tool: String,
    ) -> crate::agent::error::AgentRunResult<i64> {
        subagent::spawn_subagent_detached(self, parent, request, collab_call_id, collab_tool).await
    }

    async fn cancel_subagent(
        &self,
        _parent: &crate::agent::core::context::AgentRunContext,
        thread_id: i64,
    ) -> crate::agent::error::AgentRunResult<()> {
        if let Some(ctx) = self
            .active_runs()
            .iter()
            .find(|entry| entry.value().thread_id == thread_id)
            .map(|entry| Arc::clone(entry.value()))
        {
            ctx.abort();
        }
        self.inner
            .agent_persistence
            .threads()
            .update_status(thread_id, "cancelled")
            .await
            .map_err(|e| crate::agent::error::AgentRunError::StatePersistenceFailed(e.to_string()))
    }
}

impl AgentRunExecutor {
    /// Create new agent run executor instance.
    ///
    /// Pass `None` for `checkpoint_service` if checkpointing is not needed.
    pub fn new(
        database: Arc<DatabaseManager>,
        cache: Arc<UnifiedCache>,
        agent_persistence: Arc<AgentPersistence>,
        settings_manager: Arc<SettingsManager>,
        mcp_registry: Arc<McpRegistry>,
        lsp_manager: Arc<LspManager>,
        workspace_changes: Arc<WorkspaceChangeJournal>,
        vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    ) -> Self {
        Self::build(AgentRunExecutorServices {
            database,
            cache,
            agent_persistence,
            settings_manager,
            mcp_registry,
            lsp_manager,
            checkpoint_service: None,
            workspace_changes,
            vector_search_engine,
        })
    }

    /// Create agent run executor instance with Checkpoint service.
    pub(crate) fn with_checkpoint_service(services: AgentRunExecutorServices) -> Self {
        Self::build(services)
    }

    fn build(services: AgentRunExecutorServices) -> Self {
        let AgentRunExecutorServices {
            database,
            cache,
            agent_persistence,
            settings_manager,
            mcp_registry,
            lsp_manager,
            checkpoint_service,
            workspace_changes,
            vector_search_engine,
        } = services;
        let prompt_orchestrator = Arc::new(PromptOrchestrator::new(
            Arc::clone(&cache),
            Arc::clone(&database),
            Arc::clone(&settings_manager),
        ));
        let react_orchestrator = Arc::new(ReactOrchestrator::new(
            Arc::clone(&database),
            Arc::clone(&agent_persistence),
        ));

        Self {
            inner: Arc::new(AgentRunExecutorInner {
                database,
                cache,
                agent_persistence,
                settings_manager,
                mcp_registry,
                lsp_manager,
                checkpoint_service,
                workspace_changes,
                vector_search_engine,
                tool_confirmations: Arc::new(ToolConfirmationManager::new()),
                prompt_orchestrator,
                react_orchestrator,
                active_runs: DashMap::new(),
                active_child_executions_by_parent: DashMap::new(),
            }),
        }
    }

    pub(crate) fn vector_search_engine(
        &self,
    ) -> Option<Arc<crate::vector_db::search::SemanticSearchEngine>> {
        self.inner.vector_search_engine.clone()
    }

    // Getters for internal components

    pub fn database(&self) -> Arc<DatabaseManager> {
        Arc::clone(&self.inner.database)
    }

    pub fn cache(&self) -> Arc<UnifiedCache> {
        Arc::clone(&self.inner.cache)
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.inner.agent_persistence)
    }

    pub fn settings_manager(&self) -> Arc<SettingsManager> {
        Arc::clone(&self.inner.settings_manager)
    }

    pub fn mcp_registry(&self) -> Arc<McpRegistry> {
        Arc::clone(&self.inner.mcp_registry)
    }

    pub fn lsp_manager(&self) -> Arc<LspManager> {
        Arc::clone(&self.inner.lsp_manager)
    }

    pub fn workspace_changes(&self) -> Arc<WorkspaceChangeJournal> {
        Arc::clone(&self.inner.workspace_changes)
    }

    pub fn tool_confirmations(&self) -> Arc<ToolConfirmationManager> {
        Arc::clone(&self.inner.tool_confirmations)
    }

    pub(crate) fn prompt_orchestrator(&self) -> Arc<PromptOrchestrator> {
        Arc::clone(&self.inner.prompt_orchestrator)
    }

    pub(crate) fn react_orchestrator(&self) -> Arc<ReactOrchestrator> {
        Arc::clone(&self.inner.react_orchestrator)
    }

    pub(crate) fn active_runs(
        &self,
    ) -> &DashMap<String, Arc<crate::agent::core::context::AgentRunContext>> {
        &self.inner.active_runs
    }

    pub(crate) fn active_child_executions_global(&self) -> usize {
        self.inner
            .active_runs
            .iter()
            .filter(|entry| !entry.value().emits_task_events())
            .count()
    }

    pub(crate) fn active_child_executions_for_parent(&self, parent_task_id: &str) -> usize {
        match self
            .inner
            .active_child_executions_by_parent
            .get(parent_task_id)
        {
            Some(count) => *count,
            None => 0,
        }
    }

    pub(crate) fn increment_active_child_executions_for_parent(&self, parent_task_id: &str) {
        self.inner
            .active_child_executions_by_parent
            .entry(parent_task_id.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub(crate) fn decrement_active_child_executions_for_parent(&self, parent_task_id: &str) {
        if let Some(mut entry) = self
            .inner
            .active_child_executions_by_parent
            .get_mut(parent_task_id)
        {
            if *entry > 1 {
                *entry -= 1;
            } else {
                drop(entry);
                self.inner
                    .active_child_executions_by_parent
                    .remove(parent_task_id);
            }
        }
    }

    /// Get Checkpoint service (if configured)
    pub fn checkpoint_service(&self) -> Option<Arc<CheckpointService>> {
        self.inner.checkpoint_service.clone()
    }
}
