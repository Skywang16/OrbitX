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
mod state;
mod types;
mod react_handler;
mod react_impl;

pub use state::TaskExecutorStats;
pub use types::*;
pub use react_handler::ReactHandler;

use std::sync::Arc;

use dashmap::DashMap;

use crate::agent::persistence::AgentPersistence;
use crate::agent::prompt::orchestrator::PromptOrchestrator;
use crate::agent::react::orchestrator::ReactOrchestrator;
use crate::agent::ui::AgentUiPersistence;
use crate::storage::{DatabaseManager, UnifiedCache};

/// TaskExecutor内部状态
struct TaskExecutorInner {
    // 核心服务
    database: Arc<DatabaseManager>,
    cache: Arc<UnifiedCache>,
    agent_persistence: Arc<AgentPersistence>,
    ui_persistence: Arc<AgentUiPersistence>,

    // 编排器
    prompt_orchestrator: Arc<PromptOrchestrator>,
    react_orchestrator: Arc<ReactOrchestrator>,

    // 任务状态管理 - 使用Arc避免clone整个TaskContext
    active_tasks: DashMap<String, Arc<crate::agent::core::context::TaskContext>>,
    conversation_contexts: DashMap<i64, Arc<crate::agent::core::context::TaskContext>>,
}

/// TaskExecutor - 任务执行器
///
/// # 零成本抽象设计
/// - 使用Arc<TaskContext>而非TaskContext，避免clone
/// - 所有API返回Arc<TaskContext>，调用者自行管理生命周期
/// - DashMap直接存储Arc，取用时只增加引用计数
#[derive(Clone)]
pub struct TaskExecutor {
    inner: Arc<TaskExecutorInner>,
}

impl TaskExecutor {
    /// 创建新的TaskExecutor实例
    pub fn new(
        database: Arc<DatabaseManager>,
        cache: Arc<UnifiedCache>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
    ) -> Self {
        let prompt_orchestrator = Arc::new(PromptOrchestrator::new(Arc::clone(&cache)));
        let react_orchestrator = Arc::new(ReactOrchestrator::new(
            Arc::clone(&database),
            Arc::clone(&agent_persistence),
        ));

        Self {
            inner: Arc::new(TaskExecutorInner {
                database,
                cache,
                agent_persistence,
                ui_persistence,
                prompt_orchestrator,
                react_orchestrator,
                active_tasks: DashMap::new(),
                conversation_contexts: DashMap::new(),
            }),
        }
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

    pub fn ui_persistence(&self) -> Arc<AgentUiPersistence> {
        Arc::clone(&self.inner.ui_persistence)
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

    pub(crate) fn conversation_contexts(
        &self,
    ) -> &DashMap<i64, Arc<crate::agent::core::context::TaskContext>> {
        &self.inner.conversation_contexts
    }
}
