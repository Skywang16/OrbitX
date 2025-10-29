use std::collections::VecDeque;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::chain::Chain;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::TaskProgressPayload;
use crate::agent::persistence::{
    now_timestamp, AgentExecution, AgentPersistence, ExecutionStatus, MessageRole,
};
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::state::iteration::{IterationContext, IterationSnapshot};
use crate::agent::state::manager::{
    StateEventEmitter, StateManager, TaskState, TaskStatus, TaskThresholds,
};
use crate::agent::state::session::SessionContext;
use crate::agent::utils::tokenizer::count_text_tokens;
use crate::agent::tools::ToolRegistry;
use crate::agent::types::TaskDetail;
use crate::agent::ui::{AgentUiPersistence, UiStep};
use crate::llm::anthropic_types::{
    ContentBlock, MessageContent, MessageParam, MessageRole as AnthropicRole, SystemPrompt,
    ToolResultContent,
};
use crate::storage::DatabaseManager;

pub fn generate_node_id(task_id: &str, phase: &str, node_index: Option<usize>) -> String {
    if let Some(index) = node_index {
        format!("{}_node_{}", task_id, index)
    } else {
        format!("{}_{}", task_id, phase)
    }
}

/// 消息历史最大数量（防止内存无限增长）
const MAX_MESSAGE_HISTORY: usize = 100;

struct ExecutionState {
    record: AgentExecution,
    runtime_status: AgentTaskStatus,
    system_prompt: Option<SystemPrompt>,
    messages: VecDeque<MessageParam>,
    message_sequence: i64,
    tool_results: Vec<ToolCallResult>,
    current_iteration: Option<Arc<IterationContext>>,
    ui_assistant_message_id: Option<i64>,
}

struct PlanningState {
    chain: Chain,
    conversation: Vec<String>,
    current_node_id: Option<String>,
    task_detail: Option<TaskDetail>,
    root_task_id: Option<String>,
    parent_task_id: Option<String>,
    children: Vec<String>,
}

#[derive(Default)]
struct UiState {
    steps: Vec<UiStep>,
}

pub struct TaskContext {
    pub task_id: String,
    pub conversation_id: i64,
    pub user_prompt: String,
    pub cwd: String,

    config: TaskExecutionConfig,
    session: Arc<SessionContext>,
    tool_registry: Arc<ToolRegistry>,
    progress_channel: Arc<RwLock<Option<Channel<TaskProgressPayload>>>>,
    ui_state: Arc<Mutex<UiState>>,

    execution: Arc<RwLock<ExecutionState>>,
    planning: Arc<RwLock<PlanningState>>,

    cancellation: Arc<RwLock<CancellationToken>>,
    step_tokens: Arc<StdMutex<Vec<CancellationToken>>>,
    pause_status: Arc<AtomicU8>, // 0: running, 1: paused, 2: pause & abort current step

    react_runtime: Arc<RwLock<ReactRuntime>>,
    state_manager: Arc<StateManager>,

    // Monotonic event sequence for debugging emission order
    event_seq: Arc<AtomicU64>,
}

impl TaskContext {
    /// Create a dummy context for error handling (used in tool registry)
    /// Note: This is a minimal context only for error handling purposes
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn dummy() -> Self {
        use crate::agent::config::AgentConfig;

        let agent_config = AgentConfig::default();
        let runtime_config = ReactRuntimeConfig {
            max_iterations: agent_config.max_react_num,
            max_consecutive_errors: agent_config.max_react_error_streak,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
        };

        let dummy_registry = Arc::new(ToolRegistry::new(
            crate::agent::tools::get_permissions_for_mode("agent"),
        ));

        let dummy_execution = AgentExecution {
            id: -1,
            execution_id: "dummy".to_string(),
            conversation_id: -1,
            user_request: String::new(),
            system_prompt_used: String::new(),
            execution_config: None,
            has_conversation_context: false,
            status: ExecutionStatus::Running,
            current_iteration: 0,
            error_count: 0,
            max_iterations: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            context_tokens: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        };

        let task_state = TaskState::new("dummy".to_string(), thresholds);

        // Note: 这些字段在 dummy context 中不会被使用，只是为了满足类型系统
        // 如果真的被调用，会 panic
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let temp_app_dir =
            std::env::temp_dir().join(format!("orbitx-agent-dummy-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_app_dir).expect("Failed to create temp dir for dummy DB");
        let storage_paths = crate::storage::paths::StoragePaths::new(temp_app_dir)
            .expect("Failed to create storage paths for dummy DB");
        let dummy_db = Arc::new(
            runtime
                .block_on(crate::storage::database::DatabaseManager::new(
                    storage_paths,
                    crate::storage::database::DatabaseOptions::default(),
                ))
                .expect("Failed to create dummy DB"),
        );
        let dummy_persistence = Arc::new(AgentPersistence::new(Arc::clone(&dummy_db)));
        let dummy_ui_persistence = Arc::new(AgentUiPersistence::new(Arc::clone(&dummy_db)));

        let cwd = fallback_cwd();
        let config = TaskExecutionConfig::default();
        let session = Arc::new(SessionContext::new(
            "dummy".to_string(),
            -1,
            PathBuf::from(&cwd),
            String::new(),
            config.clone(),
            Arc::clone(&dummy_db),
            Arc::clone(&dummy_persistence),
            Arc::clone(&dummy_ui_persistence),
        ));

        let execution_state = ExecutionState {
            record: dummy_execution,
            runtime_status: AgentTaskStatus::Running,
            system_prompt: None,
            messages: VecDeque::with_capacity(MAX_MESSAGE_HISTORY),
            message_sequence: 0,
            tool_results: Vec::new(),
            current_iteration: None,
            ui_assistant_message_id: None,
        };

        let planning_state = PlanningState {
            chain: Chain::new(String::new()),
            conversation: Vec::new(),
            current_node_id: None,
            task_detail: None,
            root_task_id: None,
            parent_task_id: None,
            children: Vec::new(),
        };

        Self {
            task_id: "dummy".to_string(),
            conversation_id: -1,
            user_prompt: String::new(),
            cwd,
            config,
            session,
            tool_registry: dummy_registry,
            progress_channel: Arc::new(RwLock::new(None)),
            ui_state: Arc::new(Mutex::new(UiState::default())),
            execution: Arc::new(RwLock::new(execution_state)),
            planning: Arc::new(RwLock::new(planning_state)),
            cancellation: Arc::new(RwLock::new(CancellationToken::new())),
            step_tokens: Arc::new(StdMutex::new(Vec::new())),
            pause_status: Arc::new(AtomicU8::new(0)),
            react_runtime: Arc::new(RwLock::new(ReactRuntime::new(runtime_config))),
            state_manager: Arc::new(StateManager::new(task_state, StateEventEmitter::new())),
            event_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Construct a fresh context for a new task.
    pub async fn new(
        execution: AgentExecution,
        config: TaskExecutionConfig,
        cwd: String,
        tool_registry: Arc<ToolRegistry>,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        repositories: Arc<DatabaseManager>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
    ) -> TaskExecutorResult<Self> {
        let agent_config = AgentConfig::default();
        let runtime_config = ReactRuntimeConfig {
            max_iterations: agent_config.max_react_num,
            max_consecutive_errors: agent_config.max_react_error_streak,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
        };

        let record = execution;
        let task_id = record.execution_id.clone();
        let conversation_id = record.conversation_id;
        let user_prompt = record.user_request.clone();
        let task_status = AgentTaskStatus::from(record.status.clone());
        let current_iteration = record.current_iteration as u32;
        let error_count = record.error_count as u32;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        let execution_state = ExecutionState {
            record,
            runtime_status: task_status.clone(),
            system_prompt: None,
            messages: VecDeque::with_capacity(MAX_MESSAGE_HISTORY),
            message_sequence: 0,
            tool_results: Vec::new(),
            current_iteration: None,
            ui_assistant_message_id: None,
        };

        let planning_state = PlanningState {
            chain: Chain::new(user_prompt.clone()),
            conversation: Vec::new(),
            current_node_id: None,
            task_detail: None,
            root_task_id: None,
            parent_task_id: None,
            children: Vec::new(),
        };

        let cwd = if cwd.trim().is_empty() {
            fallback_cwd()
        } else {
            cwd
        };

        let session = Arc::new(SessionContext::new(
            task_id.clone(),
            conversation_id,
            PathBuf::from(&cwd),
            user_prompt.clone(),
            config.clone(),
            Arc::clone(&repositories),
            Arc::clone(&agent_persistence),
            Arc::clone(&ui_persistence),
        ));

        Ok(Self {
            task_id,
            conversation_id,
            user_prompt: user_prompt.clone(),
            cwd,
            config,
            session,
            tool_registry,
            progress_channel: Arc::new(RwLock::new(progress_channel)),
            ui_state: Arc::new(Mutex::new(UiState::default())),
            execution: Arc::new(RwLock::new(execution_state)),
            planning: Arc::new(RwLock::new(planning_state)),
            cancellation: Arc::new(RwLock::new(CancellationToken::new())),
            step_tokens: Arc::new(StdMutex::new(Vec::new())),
            pause_status: Arc::new(AtomicU8::new(0)),
            react_runtime: Arc::new(RwLock::new(ReactRuntime::new(runtime_config))),
            state_manager: Arc::new(StateManager::new(task_state, StateEventEmitter::new())),
            event_seq: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Attach a new progress channel (used when resuming tasks).
    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskProgressPayload>>) {
        let mut guard = self.progress_channel.write().await;
        *guard = channel;
    }

    pub async fn begin_iteration(&self, iteration_num: u32) -> Arc<IterationContext> {
        let iter_ctx = Arc::new(IterationContext::new(iteration_num, self.session()));
        let mut state = self.execution.write().await;
        state.current_iteration = Some(Arc::clone(&iter_ctx));
        iter_ctx
    }

    pub async fn end_iteration(&self) -> Option<IterationSnapshot> {
        let maybe_ctx = {
            let mut state = self.execution.write().await;
            state.current_iteration.take()
        };
        if let Some(ctx) = maybe_ctx {
            match Arc::try_unwrap(ctx) {
                Ok(inner) => Some(inner.finalize().await),
                Err(arc_ctx) => {
                    warn!("IterationContext still has outstanding references; skipping finalize");
                    drop(arc_ctx);
                    None
                }
            }
        } else {
            None
        }
    }

    pub async fn current_iteration_ctx(&self) -> Option<Arc<IterationContext>> {
        self.execution.read().await.current_iteration.clone()
    }

    pub fn session(&self) -> Arc<SessionContext> {
        Arc::clone(&self.session)
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        self.session.file_tracker()
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        self.session.agent_persistence()
    }

    pub fn ui_persistence(&self) -> Arc<AgentUiPersistence> {
        self.session.ui_persistence()
    }

    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    /// Current status of the task.
    pub async fn status(&self) -> AgentTaskStatus {
        self.execution.read().await.runtime_status.clone()
    }

    /// Set the task status and persist the change.
    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        let (execution_status, current_iteration, error_count) = {
            let mut state = self.execution.write().await;
            state.runtime_status = status.clone();
            state.record.status = ExecutionStatus::from(&status);
            (
                state.record.status.clone(),
                state.record.current_iteration,
                state.record.error_count,
            )
        };

        if matches!(
            status,
            AgentTaskStatus::Completed | AgentTaskStatus::Cancelled | AgentTaskStatus::Error
        ) {
            self.agent_persistence()
                .agent_executions()
                .mark_finished(&self.task_id, execution_status.clone())
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        } else {
            self.agent_persistence()
                .agent_executions()
                .update_status(
                    &self.task_id,
                    execution_status.clone(),
                    current_iteration,
                    error_count,
                )
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        }

        self.state_manager
            .update_task_status(map_status(&status), None)
            .await;
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> TaskExecutorResult<u32> {
        let (current, current_raw, status, errors) = {
            let mut state = self.execution.write().await;
            state.record.current_iteration = state.record.current_iteration.saturating_add(1);
            state.message_sequence = 0;
            (
                state.record.current_iteration as u32,
                state.record.current_iteration,
                state.record.status.clone(),
                state.record.error_count,
            )
        };

        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, current_raw, errors)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.state_manager.increment_iteration().await;
        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.execution.read().await.record.current_iteration as u32
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let (count, status, iteration, errors) = {
            let mut state = self.execution.write().await;
            state.record.error_count = state.record.error_count.saturating_add(1);
            (
                state.record.error_count as u32,
                state.record.status.clone(),
                state.record.current_iteration,
                state.record.error_count,
            )
        };
        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, iteration, errors)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.increment_error_count().await;
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> TaskExecutorResult<()> {
        let (status, iteration) = {
            let mut state = self.execution.write().await;
            state.record.error_count = 0;
            (state.record.status.clone(), state.record.current_iteration)
        };
        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, iteration, 0)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.reset_error_count().await;
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let (status, iteration, errors) = {
            let execution = self.execution.read().await;
            (
                execution.runtime_status.clone(),
                execution.record.current_iteration as u32,
                execution.record.error_count as u32,
            )
        };
        if matches!(
            status,
            AgentTaskStatus::Cancelled | AgentTaskStatus::Completed | AgentTaskStatus::Error
        ) {
            return true;
        }
        self.state_manager.should_halt().await
            || iteration >= self.config.max_iterations
            || errors >= self.config.max_errors
    }

    /// Access the execution configuration.
    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    /// Access repositories (used by LLM/tool bridges).
    pub fn repositories(&self) -> Arc<DatabaseManager> {
        self.session.repositories()
    }

    /// Access the React runtime.
    pub fn react_runtime(&self) -> Arc<RwLock<ReactRuntime>> {
        Arc::clone(&self.react_runtime)
    }

    /// Access the unified state manager.
    pub fn state_manager(&self) -> Arc<StateManager> {
        Arc::clone(&self.state_manager)
    }

    async fn with_planning<T>(&self, f: impl FnOnce(&PlanningState) -> T) -> T {
        let state = self.planning.read().await;
        f(&state)
    }

    async fn with_planning_mut<T>(&self, f: impl FnOnce(&mut PlanningState) -> T) -> T {
        let mut state = self.planning.write().await;
        f(&mut state)
    }

    pub async fn with_chain<T>(&self, f: impl FnOnce(&Chain) -> T) -> T {
        self.with_planning(|state| f(&state.chain)).await
    }

    pub async fn with_chain_mut<T>(&self, f: impl FnOnce(&mut Chain) -> T) -> T {
        self.with_planning_mut(|state| f(&mut state.chain)).await
    }

    /// Push a user intervention (manual conversation entry).
    pub async fn push_conversation_message(&self, message: String) {
        let mut planning = self.planning.write().await;
        planning.conversation.push(message);
    }

    /// Drain user interventions (similar to frontend behaviour).
    pub async fn drain_conversation(&self) -> Vec<String> {
        let mut planning = self.planning.write().await;
        let drained = planning.conversation.clone();
        planning.conversation.clear();
        drained
    }

    /// Update cached task detail.
    pub async fn set_task_detail(&self, task: Option<TaskDetail>) {
        let mut planning = self.planning.write().await;
        planning.task_detail = task;
    }

    pub async fn task_detail(&self) -> Option<TaskDetail> {
        self.planning.read().await.task_detail.clone()
    }

    pub async fn attach_parent(&self, parent_task_id: String, root_task_id: Option<String>) {
        let mut planning = self.planning.write().await;
        planning.parent_task_id = Some(parent_task_id.clone());
        planning.root_task_id = Some(root_task_id.unwrap_or(parent_task_id));
    }

    pub async fn add_child(&self, child_task_id: String) {
        let mut planning = self.planning.write().await;
        if !planning.children.contains(&child_task_id) {
            planning.children.push(child_task_id);
        }
    }

    /// Read current node identifier.
    pub async fn current_node_id(&self) -> Option<String> {
        self.planning.read().await.current_node_id.clone()
    }

    pub async fn set_current_node_id(&self, node_id: Option<String>) {
        let mut planning = self.planning.write().await;
        planning.current_node_id = node_id;
    }

    pub async fn check_aborted(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        {
            let cancellation = self.cancellation.read().await;
            if cancellation.is_cancelled() {
                return Err(TaskExecutorError::TaskInterrupted.into());
            }
        }
        if no_check_pause {
            return Ok(());
        }
        loop {
            {
                let cancellation = self.cancellation.read().await;
                if cancellation.is_cancelled() {
                    return Err(TaskExecutorError::TaskInterrupted.into());
                }
            }
            let status = self.pause_status.load(Ordering::SeqCst);
            if status == 0 {
                break;
            }
            if status == 2 {
                self.abort_current_steps();
                self.pause_status.store(1, Ordering::SeqCst);
            }
            sleep(Duration::from_millis(500)).await;
        }
        Ok(())
    }

    /// Abort the entire task execution.
    pub fn abort(&self) {
        if let Ok(cancellation) = self.cancellation.try_read() {
            cancellation.cancel();
        }
        self.abort_current_steps();
        if let Ok(mut runtime) = self.react_runtime.try_write() {
            runtime.mark_abort();
        }
    }

    /// 重置 cancellation token（用于中断后继续对话）
    pub async fn reset_cancellation(&self) {
        let mut cancellation = self.cancellation.write().await;
        *cancellation = CancellationToken::new();
        // 同时清空之前的 step tokens
        if let Ok(mut tokens) = self.step_tokens.lock() {
            tokens.clear();
        }
    }

    pub fn set_pause(&self, paused: bool, abort_current_step: bool) {
        let new_status = if paused {
            if abort_current_step {
                2
            } else {
                1
            }
        } else {
            0
        };
        self.pause_status.store(new_status, Ordering::SeqCst);
        if new_status == 2 {
            self.abort_current_steps();
        }
    }

    /// Register a new cancellation token for an inner step (e.g., LLM call).
    pub fn register_step_token(&self) -> CancellationToken {
        let token = if let Ok(cancellation) = self.cancellation.try_read() {
            cancellation.child_token()
        } else {
            CancellationToken::new()
        };
        if let Ok(mut tokens) = self.step_tokens.lock() {
            tokens.push(token.clone());
        }
        token
    }

    fn abort_current_steps(&self) {
        if let Ok(mut tokens) = self.step_tokens.lock() {
            for token in tokens.drain(..) {
                token.cancel();
            }
        }
    }

    /// Add assistant message using Anthropic-native types (text and/or tool uses).
    pub async fn add_assistant_message(
        &self,
        text: Option<String>,
        tool_calls: Option<Vec<ContentBlock>>,
    ) {
        let content: MessageContent = match (text, tool_calls) {
            (Some(t), Some(mut calls)) => {
                // Put text as a Text block, then append tool_use blocks
                calls.insert(
                    0,
                    ContentBlock::Text {
                        text: t,
                        cache_control: None,
                    },
                );
                MessageContent::Blocks(calls)
            }
            (Some(t), None) => MessageContent::Text(t),
            (None, Some(calls)) => MessageContent::Blocks(calls),
            (None, None) => MessageContent::Text(String::new()),
        };

        {
            let mut state = self.execution.write().await;
            // 使用VecDeque并实施容量限制
            if state.messages.len() >= MAX_MESSAGE_HISTORY {
                state.messages.pop_front();
            }
            state.messages.push_back(MessageParam {
                role: AnthropicRole::Assistant,
                content: content.clone(),
            });
        }

        // Persist assistant visible content only as a string; do not modify DB schema.
        let rendered = render_message_content(&content);
        let _ = self
            .append_message(MessageRole::Assistant, &rendered, false)
            .await;
    }

    /// Append tool results as a user message with ToolResult blocks; also persist tool rows.
    pub async fn add_tool_results(&self, results: Vec<ToolCallResult>) {
        // Build ToolResult blocks
        let blocks: Vec<ContentBlock> = results
            .iter()
            .map(|r| ContentBlock::ToolResult {
                tool_use_id: r.call_id.clone(),
                content: Some(ToolResultContent::Text(
                    serde_json::to_string(&r.result).unwrap_or_else(|_| "{}".to_string()),
                )),
                is_error: Some(r.is_error),
            })
            .collect();

        {
            let mut state = self.execution.write().await;
            state.tool_results.extend(results.clone());
            // 使用VecDeque并实施容量限制
            if state.messages.len() >= MAX_MESSAGE_HISTORY {
                state.messages.pop_front();
            }
            state.messages.push_back(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Blocks(blocks),
            });
        }

        // Persist each tool result as its own Tool message entry
        for result in results {
            if let Ok(serialized) = serde_json::to_string(&result) {
                let _ = self
                    .append_message(MessageRole::Tool, &serialized, false)
                    .await;
            }
        }
    }

    // Deprecated in zero-abstraction model: initial prompts are handled explicitly by caller.
    // Retained signature temporarily, but now implemented using set_system_prompt + add_user_message semantics without DB writes for system.
    pub async fn set_initial_prompts(
        &self,
        system_prompt: String,
        user_prompt: String,
    ) -> TaskExecutorResult<()> {
        // Set system prompt in memory only (no DB persist per requirement)
        {
            let mut state = self.execution.write().await;
            state.system_prompt = Some(SystemPrompt::Text(system_prompt));
            state.messages.clear();
            state.message_sequence = 0;
        }
        // Append the initial user message
        self.add_user_message(user_prompt).await;
        Ok(())
    }

    /// Get messages (Anthropic native) by cloning current buffer.
    pub async fn get_messages(&self) -> VecDeque<MessageParam> {
        let guard = self.execution.read().await;
        guard.messages.clone()
    }

    pub async fn get_system_prompt(&self) -> Option<SystemPrompt> {
        let guard = self.execution.read().await;
        guard.system_prompt.clone()
    }

    /// Borrow current Anthropic messages without cloning.
    pub async fn with_messages<T>(&self, f: impl FnOnce(&VecDeque<MessageParam>) -> T) -> T {
        let guard = self.execution.read().await;
        f(&guard.messages)
    }

    /// Append a user message to the conversation history (Anthropic-native types).
    pub async fn add_user_message(&self, text: String) {
        {
            let mut state = self.execution.write().await;
            let before = state.messages.len();
            // 使用VecDeque并实施容量限制
            if state.messages.len() >= MAX_MESSAGE_HISTORY {
                state.messages.pop_front();
            }
            state.messages.push_back(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Text(text.clone()),
            });
            let after = state.messages.len();
            drop(state);
            info!(
                "[TaskContext] add_user_message: messages {} -> {}",
                before, after
            );
        }
        let _ = self.append_message(MessageRole::User, &text, false).await;
    }

    /// Clear current message buffer and persisted records for this execution.
    pub async fn reset_message_state(&self) -> TaskExecutorResult<()> {
        {
            let mut state = self.execution.write().await;
            state.messages.clear();
            state.message_sequence = 0;
        }

        self.agent_persistence()
            .execution_messages()
            .delete_for_execution(&self.task_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    /// Set system prompt in memory only; do not persist system message to DB.
    pub async fn set_system_prompt(&self, prompt: String) -> TaskExecutorResult<()> {
        let mut state = self.execution.write().await;
        state.system_prompt = Some(SystemPrompt::Text(prompt));
        Ok(())
    }

    // Deprecated: system prompt is stored separately and not part of messages.
    pub async fn update_system_prompt(&self, new_system_prompt: String) -> TaskExecutorResult<()> {
        let mut state = self.execution.write().await;
        state.system_prompt = Some(SystemPrompt::Text(new_system_prompt));
        Ok(())
    }

    /// Restore messages from database (signature updated to Anthropic-native types).
    pub async fn restore_messages(&self, messages: Vec<MessageParam>) -> TaskExecutorResult<()> {
        let mut state = self.execution.write().await;
        // 直接从Vec转为VecDeque，保留最近MAX_MESSAGE_HISTORY条
        let mut deque = VecDeque::from(messages);
        while deque.len() > MAX_MESSAGE_HISTORY {
            deque.pop_front();
        }
        state.messages = deque;
        state.runtime_status = AgentTaskStatus::Completed;
        info!(
            "[TaskContext] restore_messages: Restored {} messages for task {}",
            state.messages.len(),
            self.task_id
        );
        Ok(())
    }

    async fn append_message(
        &self,
        role: MessageRole,
        content: &str,
        is_summary: bool,
    ) -> TaskExecutorResult<()> {
        let (iteration, seq) = {
            let mut state = self.execution.write().await;
            let iteration = state.record.current_iteration;
            let seq = state.message_sequence;
            state.message_sequence = seq.saturating_add(1);
            (iteration, seq)
        };

        self.agent_persistence()
            .execution_messages()
            .append_message(
                &self.task_id,
                role,
                content,
                i64::try_from(count_text_tokens(content)).unwrap_or(i64::MAX),
                is_summary,
                iteration,
                seq,
            )
            .await
            .map(|_| ())
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()).into())
    }

    pub async fn initialize_ui_track(&self, user_prompt: &str) -> TaskExecutorResult<()> {
        self.ui_persistence()
            .ensure_conversation(self.conversation_id, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.ui_persistence()
            .create_user_message(self.conversation_id, user_prompt)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let assistant_message_id = self
            .ui_persistence()
            .create_assistant_message(self.conversation_id, "streaming")
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        {
            let mut state = self.execution.write().await;
            state.ui_assistant_message_id = Some(assistant_message_id);
        }

        {
            let mut ui = self.ui_state.lock().await;
            ui.steps.clear();
        }

        Ok(())
    }

    pub async fn begin_followup_turn(&self, user_prompt: &str) -> TaskExecutorResult<()> {
        self.ui_persistence()
            .ensure_conversation(self.conversation_id, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.ui_persistence()
            .create_user_message(self.conversation_id, user_prompt)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        let assistant_message_id = self
            .ui_persistence()
            .create_assistant_message(self.conversation_id, "streaming")
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        {
            let mut state = self.execution.write().await;
            state.ui_assistant_message_id = Some(assistant_message_id);
        }

        {
            let mut ui = self.ui_state.lock().await;
            ui.steps.clear();
        }

        Ok(())
    }

    pub async fn restore_ui_track(&self) -> TaskExecutorResult<()> {
        if let Some(message) = self
            .ui_persistence()
            .get_latest_assistant_message(self.conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        {
            {
                let mut state = self.execution.write().await;
                state.ui_assistant_message_id = Some(message.id);
            }
            if let Some(steps) = message.steps {
                let mut ui = self.ui_state.lock().await;
                ui.steps = steps;
            }
        }
        Ok(())
    }

    /// Send progress payload back to the frontend if a channel is attached and update UI track.
    pub async fn send_progress(&self, payload: TaskProgressPayload) -> TaskExecutorResult<()> {
        // Debug emission ordering with a per-context sequence
        let seq = self.event_seq.fetch_add(1, Ordering::SeqCst);
        match &payload {
            TaskProgressPayload::Thinking(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "Thinking",
                    iteration = p.iteration,
                    stream_id = %p.stream_id,
                    stream_done = p.stream_done,
                    len = p.thought.len(),
                    ts = %p.timestamp,
                    "emit"
                );
            }
            TaskProgressPayload::Text(_p) => {}
            TaskProgressPayload::ToolUse(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "ToolUse",
                    iteration = p.iteration,
                    tool_id = %p.tool_id,
                    tool_name = %p.tool_name,
                    ts = %p.timestamp,
                    "emit"
                );
            }
            TaskProgressPayload::ToolResult(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "ToolResult",
                    iteration = p.iteration,
                    tool_id = %p.tool_id,
                    tool_name = %p.tool_name,
                    is_error = p.is_error,
                    ts = %p.timestamp,
                    "emit"
                );
            }
            TaskProgressPayload::Finish(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "Finish",
                    iteration = p.iteration,
                    reason = %p.finish_reason,
                    ts = %p.timestamp,
                    "emit"
                );
            }
            TaskProgressPayload::TaskStarted(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "TaskStarted",
                    iteration = p.iteration,
                    "emit"
                );
            }
            TaskProgressPayload::TaskCreated(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %p.task_id,
                    event = "TaskCreated",
                    conversation_id = p.conversation_id,
                    "emit"
                );
            }
            TaskProgressPayload::TaskCompleted(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "TaskCompleted",
                    final_iteration = p.final_iteration,
                    ts = %p.timestamp,
                    "emit"
                );
            }
            TaskProgressPayload::TaskPaused(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "TaskPaused", reason = %p.reason, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::TaskResumed(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "TaskResumed", from_iteration = p.from_iteration, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::TaskError(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "TaskError", iteration = p.iteration, ty = %p.error_type, msg = %p.error_message, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::TaskCancelled(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "TaskCancelled", reason = %p.reason, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::StatusChanged(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "StatusChanged", status = ?p.status, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::StatusUpdate(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "StatusUpdate", status = %p.status, it = p.current_iteration, err = p.error_count, ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::SystemMessage(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "SystemMessage", level = %p.level, "emit");
            }
            TaskProgressPayload::FinalAnswer(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "FinalAnswer", iteration = p.iteration, len = p.answer.len(), ts = %p.timestamp, "emit");
            }
            TaskProgressPayload::ToolPreparing(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "ToolPreparing", tool_name = %p.tool_name, conf = p.confidence, "emit");
            }
            TaskProgressPayload::Error(p) => {
                info!(target: "task::event", seq, task_id = %self.task_id, event = "Error", msg = %p.message, recoverable = p.recoverable, "emit");
            }
        }
        {
            let guard = self.progress_channel.read().await;
            if let Some(channel) = guard.as_ref() {
                channel
                    .send(payload.clone())
                    .map_err(TaskExecutorError::ChannelError)?;
            }
        }

        self.update_ui_track(&payload).await?;
        Ok(())
    }

    async fn update_ui_track(&self, payload: &TaskProgressPayload) -> TaskExecutorResult<()> {
        let mut maybe_step = Self::step_from_payload(payload);
        let status_override = Self::status_from_payload(payload);

        if let Some(step) = maybe_step.as_ref() {
            let stream_id = step
                .metadata
                .as_ref()
                .and_then(|meta| meta.get("streamId"))
                .and_then(|v| v.as_str());
            let stream_done = step
                .metadata
                .as_ref()
                .and_then(|meta| meta.get("streamDone"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if stream_done {
                debug!(
                    target: "task::executor::ui",
                    task_id = %self.task_id,
                    conversation_id = self.conversation_id,
                    step_type = %step.step_type,
                    content_len = step.content.len(),
                    stream_id,
                    "ui stream closed"
                );
            }
        }

        if let Some(status) = status_override {
            debug!(
                target: "task::executor::ui",
                task_id = %self.task_id,
                conversation_id = self.conversation_id,
                status,
                "ui status override"
            );
        }

        if maybe_step.is_none() && status_override.is_none() {
            return Ok(());
        }

        let steps_snapshot = {
            let mut ui = self.ui_state.lock().await;
            if let Some(step) = maybe_step.take() {
                // 对thinking/text根据streamId合并，其他直接push
                let stream_id = step
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("streamId"))
                    .and_then(|v| v.as_str());

                if let Some(sid) = stream_id {
                    if let Some(existing) = ui.steps.iter_mut().find(|s| {
                        s.step_type == step.step_type
                            && s.metadata
                                .as_ref()
                                .and_then(|m| m.get("streamId"))
                                .and_then(|v| v.as_str())
                                == Some(sid)
                    }) {
                        // 合并：拼接内容+更新metadata
                        existing.content.push_str(&step.content);
                        existing.timestamp = step.timestamp;
                        existing.metadata = step.metadata;
                    } else {
                        // 新流，push
                        ui.steps.push(step);
                    }
                } else {
                    // 无streamId，直接push
                    ui.steps.push(step);
                }
            }
            ui.steps.clone()
        };

        let status = status_override.unwrap_or("streaming");
        self.persist_ui_steps(&steps_snapshot, status).await
    }

    async fn persist_ui_steps(&self, steps: &[UiStep], status: &str) -> TaskExecutorResult<()> {
        let current_id = {
            let state = self.execution.read().await;
            state.ui_assistant_message_id
        };

        if let Some(message_id) = current_id {
            self.ui_persistence()
                .update_assistant_message(message_id, steps, status)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        } else {
            return Err(TaskExecutorError::StatePersistenceFailed(
                "assistant message id not set before persisting UI steps".to_string(),
            )
            .into());
        }

        Ok(())
    }

    fn step_from_payload(payload: &TaskProgressPayload) -> Option<UiStep> {
        match payload {
            TaskProgressPayload::Thinking(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "thinking".to_string(),
                    content: p.thought.clone(),
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "streamId": p.stream_id,
                        "streamDone": p.stream_done,
                    })),
                })
            }
            TaskProgressPayload::Text(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "text".to_string(),
                    content: p.text.clone(),
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "streamId": p.stream_id,
                        "streamDone": p.stream_done,
                    })),
                })
            }
            TaskProgressPayload::ToolUse(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "tool_use".to_string(),
                    content: format!("调用工具: {}", p.tool_name),
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "toolId": p.tool_id,
                        "toolName": p.tool_name,
                        "params": p.params,
                    })),
                })
            }
            TaskProgressPayload::ToolResult(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "tool_result".to_string(),
                    content: if p.is_error {
                        format!("工具 {} 执行失败", p.tool_name)
                    } else {
                        format!("工具 {} 返回结果", p.tool_name)
                    },
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "toolId": p.tool_id,
                        "toolName": p.tool_name,
                        "result": p.result,
                        "isError": p.is_error,
                    })),
                })
            }
            TaskProgressPayload::FinalAnswer(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "text".to_string(),
                    content: p.answer.clone(),
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "final": true,
                    })),
                })
            }
            TaskProgressPayload::TaskError(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "error".to_string(),
                    content: format!("任务错误: {}", p.error_message),
                    timestamp: ts,
                    metadata: Some(json!({
                        "iteration": p.iteration,
                        "errorType": p.error_type,
                        "recoverable": p.is_recoverable,
                    })),
                })
            }
            TaskProgressPayload::Error(p) => {
                let ts = now_timestamp();
                Some(UiStep {
                    step_type: "error".to_string(),
                    content: p.message.clone(),
                    timestamp: ts,
                    metadata: None,
                })
            }
            TaskProgressPayload::SystemMessage(p) => {
                let ts = p.timestamp.timestamp_millis();
                Some(UiStep {
                    step_type: "text".to_string(),
                    content: p.message.clone(),
                    timestamp: ts,
                    metadata: Some(json!({
                        "kind": "system",
                        "level": p.level,
                    })),
                })
            }
            _ => None,
        }
    }

    fn status_from_payload(payload: &TaskProgressPayload) -> Option<&'static str> {
        match payload {
            TaskProgressPayload::TaskCompleted(_)
            | TaskProgressPayload::Finish(_)
            | TaskProgressPayload::TaskCancelled(_) => Some("complete"),
            TaskProgressPayload::TaskError(_) | TaskProgressPayload::Error(_) => Some("error"),
            _ => None,
        }
    }
}

fn map_status(status: &AgentTaskStatus) -> TaskStatus {
    match status {
        AgentTaskStatus::Created => TaskStatus::Init,
        AgentTaskStatus::Running => TaskStatus::Running,
        AgentTaskStatus::Paused => TaskStatus::Paused,
        AgentTaskStatus::Completed => TaskStatus::Done,
        AgentTaskStatus::Error => TaskStatus::Error,
        AgentTaskStatus::Cancelled => TaskStatus::Aborted,
    }
}

fn render_message_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Blocks(blocks) => serde_json::to_string(blocks).unwrap_or_default(),
    }
}

/// Normalised tool call result stored in the task context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub call_id: String,
    pub tool_name: String,
    pub result: Value,
    pub is_error: bool,
    pub execution_time_ms: u64,
}

fn fallback_cwd() -> String {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string())
}
