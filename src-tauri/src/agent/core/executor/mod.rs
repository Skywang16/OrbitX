/*!
 * TaskExecutor - Agent任务执行器
 *
 * 职责：
 * - 任务生命周期管理（创建、暂停、恢复、取消）
 * - 任务状态查询和管理
 * - 协调 PromptOrchestrator 和 ReactOrchestrator
 *
 * 非职责（已分离）：
 * - Prompt构建 -> agent/prompt/orchestrator.rs
 * - ReAct循环 -> agent/react/orchestrator/mod.rs
 * - 工具执行 -> agent/tools/
 * - 持久化 -> agent/persistence/
 */

mod builder;
mod lifecycle;
mod react_handler;
mod react_impl;
mod state;
mod subtask;
mod types;

pub use react_handler::ReactHandler;
pub use state::TaskExecutorStats;
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
use crate::settings::SettingsManager;
use crate::storage::{DatabaseManager, UnifiedCache};

/// TaskExecutor内部状态
struct TaskExecutorInner {
    // 核心服务
    database: Arc<DatabaseManager>,
    cache: Arc<UnifiedCache>,
    agent_persistence: Arc<AgentPersistence>,
    settings_manager: Arc<SettingsManager>,
    mcp_registry: Arc<McpRegistry>,

    // Checkpoint 服务（可选，用于自动创建 checkpoint）
    checkpoint_service: Option<Arc<CheckpointService>>,
    workspace_changes: Arc<WorkspaceChangeJournal>,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    tool_confirmations: Arc<ToolConfirmationManager>,

    // 编排器
    prompt_orchestrator: Arc<PromptOrchestrator>,
    react_orchestrator: Arc<ReactOrchestrator>,

    // 任务状态管理 - 仅用于查找正在运行的任务以便中断
    // 不再缓存 conversation_contexts，每次从 DB 加载
    active_tasks: DashMap<String, Arc<crate::agent::core::context::TaskContext>>,
}

/// TaskExecutor - 任务执行器
///
/// - 所有API返回Arc<TaskContext>，调用者自行管理生命周期
/// - DashMap直接存储Arc，取用时只增加引用计数
#[derive(Clone)]
pub struct TaskExecutor {
    inner: Arc<TaskExecutorInner>,
}

#[async_trait::async_trait]
impl crate::agent::core::context::SubtaskRunner for TaskExecutor {
    async fn run_subtask(
        &self,
        parent: &crate::agent::core::context::TaskContext,
        request: crate::agent::core::context::SubtaskRequest,
    ) -> crate::agent::error::TaskExecutorResult<crate::agent::core::context::SubtaskResponse> {
        subtask::run_subtask(self, parent, request).await
    }
}

impl TaskExecutor {
    /// 创建新的TaskExecutor实例
    pub fn new(
        database: Arc<DatabaseManager>,
        cache: Arc<UnifiedCache>,
        agent_persistence: Arc<AgentPersistence>,
        settings_manager: Arc<SettingsManager>,
        mcp_registry: Arc<McpRegistry>,
        workspace_changes: Arc<WorkspaceChangeJournal>,
        vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    ) -> Self {
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
            inner: Arc::new(TaskExecutorInner {
                database,
                cache,
                agent_persistence,
                settings_manager,
                mcp_registry,
                checkpoint_service: None,
                workspace_changes,
                vector_search_engine,
                tool_confirmations: Arc::new(ToolConfirmationManager::new()),
                prompt_orchestrator,
                react_orchestrator,
                active_tasks: DashMap::new(),
            }),
        }
    }

    /// 创建带有 Checkpoint 服务的 TaskExecutor 实例
    pub fn with_checkpoint_service(
        database: Arc<DatabaseManager>,
        cache: Arc<UnifiedCache>,
        agent_persistence: Arc<AgentPersistence>,
        settings_manager: Arc<SettingsManager>,
        mcp_registry: Arc<McpRegistry>,
        checkpoint_service: Arc<CheckpointService>,
        workspace_changes: Arc<WorkspaceChangeJournal>,
        vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
    ) -> Self {
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
            inner: Arc::new(TaskExecutorInner {
                database,
                cache,
                agent_persistence,
                settings_manager,
                mcp_registry,
                checkpoint_service: Some(checkpoint_service),
                workspace_changes,
                vector_search_engine,
                tool_confirmations: Arc::new(ToolConfirmationManager::new()),
                prompt_orchestrator,
                react_orchestrator,
                active_tasks: DashMap::new(),
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

    pub(crate) fn active_tasks(
        &self,
    ) -> &DashMap<String, Arc<crate::agent::core::context::TaskContext>> {
        &self.inner.active_tasks
    }

    /// 获取 Checkpoint 服务（如果已配置）
    pub fn checkpoint_service(&self) -> Option<Arc<CheckpointService>> {
        self.inner.checkpoint_service.clone()
    }
}
