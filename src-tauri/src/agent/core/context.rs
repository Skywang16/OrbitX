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
use tracing::{debug, info};

use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::chain::Chain;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::events::TaskProgressPayload;
use crate::agent::persistence::{
    now_timestamp, AgentExecution, AgentPersistence, ExecutionStatus, MessageRole,
};
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::state::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::state::manager::{
    StateEventEmitter, StateManager, TaskState, TaskStatus, TaskThresholds,
};
use crate::agent::types::{PlannedTask, TaskDetail};
use crate::agent::ui::{AgentUiPersistence, UiStep};
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart, LLMToolCall};
use crate::storage::repositories::RepositoryManager;

const SUMMARY_RECENT_MESSAGE_COUNT: usize = 3;

/// Generate a context node identifier identical to the frontend implementation.
pub fn generate_node_id(task_id: &str, phase: &str, node_index: Option<usize>) -> String {
    if let Some(index) = node_index {
        format!("{}_node_{}", task_id, index)
    } else {
        format!("{}_{}", task_id, phase)
    }
}

/// Runtime execution context for a single agent task (mirrors eko-core semantics).
pub struct TaskContext {
    pub task_id: String,
    pub conversation_id: i64,
    pub user_prompt: String,

    config: TaskExecutionConfig,
    repositories: Arc<RepositoryManager>,
    agent_persistence: Arc<AgentPersistence>,
    ui_persistence: Arc<AgentUiPersistence>,
    file_tracker: Arc<FileContextTracker>,
    progress_channel: Arc<RwLock<Option<Channel<TaskProgressPayload>>>>,
    ui_steps: Arc<Mutex<Vec<UiStep>>>,
    ui_assistant_message_id: Arc<RwLock<Option<i64>>>,

    execution_record: Arc<RwLock<AgentExecution>>,
    runtime_status: Arc<RwLock<AgentTaskStatus>>,
    messages: Arc<RwLock<Vec<LLMMessage>>>,
    message_sequence: Arc<RwLock<i64>>,
    tool_results: Arc<RwLock<Vec<ToolCallResult>>>,

    chain: Arc<RwLock<Chain>>,
    conversation: Arc<RwLock<Vec<String>>>,
    current_node_id: Arc<RwLock<Option<String>>>,
    task_detail: Arc<RwLock<Option<TaskDetail>>>,
    planned_tree: Arc<RwLock<Option<PlannedTask>>>,

    cancellation: CancellationToken,
    step_tokens: Arc<StdMutex<Vec<CancellationToken>>>,
    pause_status: Arc<AtomicU8>, // 0: running, 1: paused, 2: pause & abort current step

    react_runtime: Arc<RwLock<ReactRuntime>>,
    state_manager: Arc<RwLock<StateManager>>,

    root_task_id: Arc<RwLock<Option<String>>>,
    parent_task_id: Arc<RwLock<Option<String>>>,
    children: Arc<RwLock<Vec<String>>>,

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
            max_idle_rounds: agent_config.max_react_idle_rounds,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
            max_idle_rounds: agent_config.max_react_idle_rounds,
        };

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
        let dummy_repos = Arc::new(RepositoryManager::new(Arc::clone(&dummy_db)));
        let dummy_persistence = Arc::new(AgentPersistence::new(Arc::clone(&dummy_db)));
        let dummy_ui_persistence = Arc::new(AgentUiPersistence::new(Arc::clone(&dummy_db)));

        Self {
            task_id: "dummy".to_string(),
            conversation_id: -1,
            user_prompt: String::new(),
            config: TaskExecutionConfig::default(),
            repositories: dummy_repos,
            agent_persistence: Arc::clone(&dummy_persistence),
            ui_persistence: dummy_ui_persistence,
            file_tracker: Arc::new(FileContextTracker::new(dummy_persistence, -1)),
            progress_channel: Arc::new(RwLock::new(None)),
            ui_steps: Arc::new(Mutex::new(Vec::new())),
            ui_assistant_message_id: Arc::new(RwLock::new(None)),
            execution_record: Arc::new(RwLock::new(dummy_execution)),
            runtime_status: Arc::new(RwLock::new(AgentTaskStatus::Running)),
            messages: Arc::new(RwLock::new(Vec::new())),
            message_sequence: Arc::new(RwLock::new(0)),
            tool_results: Arc::new(RwLock::new(Vec::new())),
            chain: Arc::new(RwLock::new(Chain::new(String::new()))),
            conversation: Arc::new(RwLock::new(Vec::new())),
            current_node_id: Arc::new(RwLock::new(None)),
            task_detail: Arc::new(RwLock::new(None)),
            planned_tree: Arc::new(RwLock::new(None)),
            cancellation: CancellationToken::new(),
            step_tokens: Arc::new(StdMutex::new(Vec::new())),
            pause_status: Arc::new(AtomicU8::new(0)),
            react_runtime: Arc::new(RwLock::new(ReactRuntime::new(runtime_config))),
            state_manager: Arc::new(RwLock::new(StateManager::new(
                task_state,
                StateEventEmitter::new(),
            ))),
            root_task_id: Arc::new(RwLock::new(None)),
            parent_task_id: Arc::new(RwLock::new(None)),
            children: Arc::new(RwLock::new(Vec::new())),
            event_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Construct a fresh context for a new task.
    pub async fn new(
        execution: AgentExecution,
        config: TaskExecutionConfig,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        repositories: Arc<RepositoryManager>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
    ) -> TaskExecutorResult<Self> {
        let agent_config = AgentConfig::default();
        let runtime_config = ReactRuntimeConfig {
            max_iterations: agent_config.max_react_num,
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_idle_rounds: agent_config.max_react_idle_rounds,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
            max_idle_rounds: agent_config.max_react_idle_rounds,
        };

        let task_id = execution.execution_id.clone();
        let conversation_id = execution.conversation_id;
        let user_prompt = execution.user_request.clone();
        let task_status = AgentTaskStatus::from(execution.status.clone());
        let current_iteration = execution.current_iteration as u32;
        let error_count = execution.error_count as u32;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        let file_tracker = Arc::new(FileContextTracker::new(
            Arc::clone(&agent_persistence),
            conversation_id,
        ));

        Ok(Self {
            task_id,
            conversation_id,
            user_prompt: user_prompt.clone(),
            config,
            repositories,
            agent_persistence,
            ui_persistence,
            file_tracker,
            progress_channel: Arc::new(RwLock::new(progress_channel)),
            ui_steps: Arc::new(Mutex::new(Vec::new())),
            ui_assistant_message_id: Arc::new(RwLock::new(None)),
            execution_record: Arc::new(RwLock::new(execution)),
            runtime_status: Arc::new(RwLock::new(task_status)),
            messages: Arc::new(RwLock::new(Vec::new())),
            message_sequence: Arc::new(RwLock::new(0)),
            tool_results: Arc::new(RwLock::new(Vec::new())),
            chain: Arc::new(RwLock::new(Chain::new(user_prompt))),
            conversation: Arc::new(RwLock::new(Vec::new())),
            current_node_id: Arc::new(RwLock::new(None)),
            task_detail: Arc::new(RwLock::new(None)),
            planned_tree: Arc::new(RwLock::new(None)),
            cancellation: CancellationToken::new(),
            step_tokens: Arc::new(StdMutex::new(Vec::new())),
            pause_status: Arc::new(AtomicU8::new(0)),
            react_runtime: Arc::new(RwLock::new(ReactRuntime::new(runtime_config))),
            state_manager: Arc::new(RwLock::new(StateManager::new(
                task_state,
                StateEventEmitter::new(),
            ))),
            root_task_id: Arc::new(RwLock::new(None)),
            parent_task_id: Arc::new(RwLock::new(None)),
            children: Arc::new(RwLock::new(Vec::new())),
            event_seq: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Restore a context from persistence (task + latest snapshot).
    pub async fn restore(
        execution_id: String,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        repositories: Arc<RepositoryManager>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
    ) -> TaskExecutorResult<Self> {
        let execution = agent_persistence
            .agent_executions()
            .get_by_execution_id(&execution_id)
            .await
            .map_err(|e| TaskExecutorError::ContextRecoveryFailed(e.to_string()))?
            .ok_or_else(|| TaskExecutorError::TaskNotFound(execution_id.clone()))?;

        let config = execution
            .execution_config
            .as_ref()
            .and_then(|json| serde_json::from_str::<TaskExecutionConfig>(json).ok())
            .unwrap_or_default();

        let context = Self::new(
            execution,
            config,
            progress_channel,
            repositories,
            agent_persistence,
            ui_persistence,
        )
        .await?;
        context.load_messages_from_snapshot().await?;
        Ok(context)
    }

    /// Attach a new progress channel (used when resuming tasks).
    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskProgressPayload>>) {
        let mut guard = self.progress_channel.write().await;
        *guard = channel;
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        Arc::clone(&self.file_tracker)
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.agent_persistence)
    }

    pub fn ui_persistence(&self) -> Arc<AgentUiPersistence> {
        Arc::clone(&self.ui_persistence)
    }

    /// Current status of the task.
    pub async fn status(&self) -> AgentTaskStatus {
        self.runtime_status.read().await.clone()
    }

    /// Set the task status and persist the change.
    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        {
            let mut runtime = self.runtime_status.write().await;
            *runtime = status.clone();
        }

        let mut execution = self.execution_record.write().await;
        execution.status = ExecutionStatus::from(&status);

        if matches!(
            status,
            AgentTaskStatus::Completed | AgentTaskStatus::Cancelled | AgentTaskStatus::Error
        ) {
            self.agent_persistence
                .agent_executions()
                .mark_finished(&execution.execution_id, execution.status.clone())
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        } else {
            self.agent_persistence
                .agent_executions()
                .update_status(
                    &execution.execution_id,
                    execution.status.clone(),
                    execution.current_iteration,
                    execution.error_count,
                )
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        }

        {
            let mut manager = self.state_manager.write().await;
            manager.update_task_status(map_status(&status), None);
        }
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> TaskExecutorResult<u32> {
        let mut execution = self.execution_record.write().await;
        execution.current_iteration = execution.current_iteration.saturating_add(1);
        let current = execution.current_iteration as u32;
        self.agent_persistence
            .agent_executions()
            .update_status(
                &execution.execution_id,
                execution.status.clone(),
                execution.current_iteration,
                execution.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut sequence = self.message_sequence.write().await;
            *sequence = 0;
        }

        self.state_manager.write().await.increment_iteration();
        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.execution_record.read().await.current_iteration as u32
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let mut execution = self.execution_record.write().await;
        execution.error_count = execution.error_count.saturating_add(1);
        let count = execution.error_count as u32;
        self.agent_persistence
            .agent_executions()
            .update_status(
                &execution.execution_id,
                execution.status.clone(),
                execution.current_iteration,
                execution.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.write().await.increment_error_count();
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> TaskExecutorResult<()> {
        let mut execution = self.execution_record.write().await;
        execution.error_count = 0;
        self.agent_persistence
            .agent_executions()
            .update_status(
                &execution.execution_id,
                execution.status.clone(),
                execution.current_iteration,
                execution.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.write().await.reset_error_count();
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let execution = self.execution_record.read().await;
        let status = self.runtime_status.read().await.clone();
        if matches!(
            status,
            AgentTaskStatus::Cancelled | AgentTaskStatus::Completed | AgentTaskStatus::Error
        ) {
            return true;
        }
        let manager = self.state_manager.read().await;
        manager.should_halt()
            || (execution.current_iteration as u32) >= self.config.max_iterations
            || (execution.error_count as u32) >= self.config.max_errors
    }

    /// Access the execution configuration.
    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    /// Access repositories (used by LLM/tool bridges).
    pub fn repositories(&self) -> Arc<RepositoryManager> {
        Arc::clone(&self.repositories)
    }

    /// Access the tool chain snapshot.
    pub fn chain(&self) -> Arc<RwLock<Chain>> {
        Arc::clone(&self.chain)
    }

    /// Access the React runtime.
    pub fn react_runtime(&self) -> Arc<RwLock<ReactRuntime>> {
        Arc::clone(&self.react_runtime)
    }

    /// Access the unified state manager.
    pub fn state_manager(&self) -> Arc<RwLock<StateManager>> {
        Arc::clone(&self.state_manager)
    }

    /// Push a user intervention (manual conversation entry).
    pub async fn push_conversation_message(&self, message: String) {
        let mut conversation = self.conversation.write().await;
        conversation.push(message);
    }

    /// Drain user interventions (similar to frontend behaviour).
    pub async fn drain_conversation(&self) -> Vec<String> {
        let mut conversation = self.conversation.write().await;
        let drained = conversation.clone();
        conversation.clear();
        drained
    }

    /// Update cached task detail generated by planner.
    pub async fn set_task_detail(&self, task: Option<TaskDetail>) {
        let mut guard = self.task_detail.write().await;
        *guard = task;
    }

    pub async fn task_detail(&self) -> Option<TaskDetail> {
        self.task_detail.read().await.clone()
    }

    /// Update cached planned task tree.
    pub async fn set_planned_tree(&self, tree: Option<PlannedTask>) {
        let mut guard = self.planned_tree.write().await;
        *guard = tree;
    }

    pub async fn planned_tree(&self) -> Option<PlannedTask> {
        self.planned_tree.read().await.clone()
    }

    /// Attach parent/root task ids (mirrors frontend Context::attachParent).
    pub async fn attach_parent(&self, parent_task_id: String, root_task_id: Option<String>) {
        {
            let mut parent = self.parent_task_id.write().await;
            *parent = Some(parent_task_id.clone());
        }
        {
            let mut root = self.root_task_id.write().await;
            *root = Some(root_task_id.unwrap_or(parent_task_id.clone()));
        }
    }

    /// Track child task ids (mirrors frontend Context::addChild).
    pub async fn add_child(&self, child_task_id: String) {
        let mut children = self.children.write().await;
        if !children.contains(&child_task_id) {
            children.push(child_task_id);
        }
    }

    /// Read current node identifier.
    pub async fn current_node_id(&self) -> Option<String> {
        self.current_node_id.read().await.clone()
    }

    pub async fn set_current_node_id(&self, node_id: Option<String>) {
        let mut guard = self.current_node_id.write().await;
        *guard = node_id;
    }

    /// Check for task abort or pause status (mirrors frontend semantics).
    pub async fn check_aborted(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self.cancellation.is_cancelled() {
            return Err(TaskExecutorError::TaskInterrupted.into());
        }
        if no_check_pause {
            return Ok(());
        }
        loop {
            if self.cancellation.is_cancelled() {
                return Err(TaskExecutorError::TaskInterrupted.into());
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
        self.cancellation.cancel();
        self.abort_current_steps();
        if let Ok(mut runtime) = self.react_runtime.try_write() {
            runtime.mark_abort();
        }
    }

    /// Request a pause state identical to the frontend behaviour.
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
        let token = self.cancellation.child_token();
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

    /// Add assistant response (thinking/tool calls/final answer) to message history.
    pub async fn add_llm_response(&self, parsed: LLMResponseParsed) {
        let LLMResponseParsed {
            thinking,
            tool_calls,
            final_answer,
            raw_content,
        } = parsed;

        let mut messages = self.messages.write().await;
        let mut parts: Vec<LLMMessagePart> = Vec::new();

        if let Some(thinking_text) = thinking {
            parts.push(LLMMessagePart::Text {
                text: thinking_text,
            });
        }

        if let Some(tool_calls) = tool_calls {
            for call in tool_calls {
                parts.push(LLMMessagePart::ToolCall {
                    tool_call_id: call.id,
                    tool_name: call.name,
                    args: call.arguments,
                });
            }
        }

        if let Some(answer_text) = final_answer {
            parts.push(LLMMessagePart::Text { text: answer_text });
        }

        let content = if parts.is_empty() {
            LLMMessageContent::Text(raw_content.clone())
        } else {
            LLMMessageContent::Parts(parts)
        };

        messages.push(LLMMessage {
            role: "assistant".to_string(),
            content,
        });
        drop(messages);
        let _ = self
            .append_message(MessageRole::Assistant, &raw_content, false)
            .await;
    }

    /// Append a tool result both to structured state and conversation history.
    pub async fn add_tool_result(&self, result: ToolCallResult) {
        {
            let mut results = self.tool_results.write().await;
            results.push(result.clone());
        }

        let mut messages = self.messages.write().await;
        messages.push(LLMMessage {
            role: "tool".to_string(),
            content: LLMMessageContent::Parts(vec![LLMMessagePart::ToolResult {
                tool_call_id: result.call_id.clone(),
                tool_name: result.tool_name.clone(),
                result: result.result.clone(),
            }]),
        });
        drop(messages);
        if let Ok(serialized) = serde_json::to_string(&result) {
            let _ = self
                .append_message(MessageRole::Assistant, &serialized, false)
                .await;
        }
    }

    /// Replace initial prompts (system + user) and reset history.
    pub async fn set_initial_prompts(
        &self,
        system_prompt: String,
        user_prompt: String,
    ) -> TaskExecutorResult<()> {
        let mut messages = self.messages.write().await;
        messages.clear();
        messages.push(LLMMessage {
            role: "system".to_string(),
            content: LLMMessageContent::Text(system_prompt.clone()),
        });
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text(user_prompt.clone()),
        });
        {
            let mut sequence = self.message_sequence.write().await;
            *sequence = 0;
        }
        self.append_message(MessageRole::System, &system_prompt, false)
            .await?;
        self.append_message(MessageRole::User, &user_prompt, false)
            .await?;
        Ok(())
    }

    /// Snapshot messages for LLM request construction.
    pub async fn get_messages(&self) -> Vec<LLMMessage> {
        self.messages.read().await.clone()
    }

    /// Copy messages into the provided buffer, allowing callers to reuse an allocation.
    pub async fn copy_messages_into(&self, target: &mut Vec<LLMMessage>) {
        let guard = self.messages.read().await;
        target.clear();
        target.extend(guard.iter().cloned());
    }

    /// Append a user message to the conversation history (without resetting prompts).
    pub async fn push_user_message(&self, text: String) {
        let mut messages = self.messages.write().await;
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text(text.clone()),
        });
        let _ = self.append_message(MessageRole::User, &text, false).await;
    }

    /// Clear current message buffer and persisted records for this execution.
    pub async fn reset_message_state(&self) -> TaskExecutorResult<()> {
        {
            let mut guard = self.messages.write().await;
            guard.clear();
        }
        {
            let mut sequence = self.message_sequence.write().await;
            *sequence = 0;
        }

        self.agent_persistence
            .execution_messages()
            .delete_for_execution(&self.task_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    /// Append a system message to the conversation history (without resetting prompts).
    pub async fn push_system_message(&self, text: String) {
        let mut messages = self.messages.write().await;
        messages.push(LLMMessage {
            role: "system".to_string(),
            content: LLMMessageContent::Text(text.clone()),
        });
        let _ = self.append_message(MessageRole::System, &text, false).await;
    }

    /// Persist latest message state to the repository snapshot table.
    pub async fn save_context_snapshot(&self) -> TaskExecutorResult<()> {
        let messages = self.messages.read().await;
        let mut total_input_tokens: i64 = 0;
        let mut total_output_tokens: i64 = 0;
        let mut context_tokens: i64 = 0;

        for message in messages.iter() {
            let tokens = estimate_message_tokens(&render_llm_message(&message.content));
            context_tokens = context_tokens.saturating_add(tokens);
            match message.role.as_str() {
                "assistant" => {
                    total_output_tokens = total_output_tokens.saturating_add(tokens);
                }
                "system" | "user" => {
                    total_input_tokens = total_input_tokens.saturating_add(tokens);
                }
                _ => {}
            }
        }
        drop(messages);

        {
            let mut execution = self.execution_record.write().await;
            execution.total_input_tokens = total_input_tokens;
            execution.total_output_tokens = total_output_tokens;
            execution.context_tokens = context_tokens;
        }

        let execution = self.execution_record.read().await;
        self.agent_persistence
            .agent_executions()
            .update_token_usage(
                &execution.execution_id,
                total_input_tokens,
                total_output_tokens,
                context_tokens,
                execution.total_cost,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    pub async fn apply_conversation_summary(&self, summary: &str) -> TaskExecutorResult<()> {
        {
            let mut execution = self.execution_record.write().await;
            execution.has_conversation_context = true;
        }

        let mut messages = self.messages.write().await;
        let keep_count = SUMMARY_RECENT_MESSAGE_COUNT.min(messages.len());
        let mut tail = if keep_count == 0 {
            Vec::new()
        } else {
            let split_point = messages.len() - keep_count;
            messages.split_off(split_point)
        };

        let summary_message = LLMMessage {
            role: "system".to_string(),
            content: LLMMessageContent::Text(format!(
                "Conversation summary (auto-generated):\n{}",
                summary
            )),
        };

        let mut next_messages = Vec::with_capacity(1 + tail.len());
        next_messages.push(summary_message);
        next_messages.append(&mut tail);
        *messages = next_messages;
        drop(messages);

        {
            let mut sequence = self.message_sequence.write().await;
            *sequence = 0;
        }

        self.append_message(MessageRole::System, summary, true)
            .await?;
        self.agent_persistence
            .agent_executions()
            .set_has_context(&self.task_id, true)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    async fn append_message(
        &self,
        role: MessageRole,
        content: &str,
        is_summary: bool,
    ) -> TaskExecutorResult<()> {
        let iteration = self.execution_record.read().await.current_iteration;
        let mut sequence = self.message_sequence.write().await;
        let seq = *sequence;
        *sequence = seq.saturating_add(1);

        self.agent_persistence
            .execution_messages()
            .append_message(
                &self.task_id,
                role,
                content,
                estimate_message_tokens(content) as i64,
                is_summary,
                iteration,
                seq,
            )
            .await
            .map(|_| ())
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()).into())
    }

    pub async fn initialize_ui_track(&self, user_prompt: &str) -> TaskExecutorResult<()> {
        self.ui_persistence
            .ensure_conversation(self.conversation_id, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.ui_persistence
            .create_user_message(self.conversation_id, user_prompt)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let assistant_message_id = self
            .ui_persistence
            .upsert_assistant_message(self.conversation_id, &[], "streaming")
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        {
            let mut guard = self.ui_assistant_message_id.write().await;
            *guard = Some(assistant_message_id);
        }

        {
            let mut steps = self.ui_steps.lock().await;
            steps.clear();
        }

        Ok(())
    }

    pub async fn begin_followup_turn(&self, user_prompt: &str) -> TaskExecutorResult<()> {
        self.ui_persistence
            .ensure_conversation(self.conversation_id, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.ui_persistence
            .create_user_message(self.conversation_id, user_prompt)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let assistant_message_id = self
            .ui_persistence
            .upsert_assistant_message(self.conversation_id, &[], "streaming")
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        {
            let mut guard = self.ui_assistant_message_id.write().await;
            *guard = Some(assistant_message_id);
        }

        {
            let mut steps = self.ui_steps.lock().await;
            steps.clear();
        }

        Ok(())
    }

    pub async fn restore_ui_track(&self) -> TaskExecutorResult<()> {
        if let Some(message) = self
            .ui_persistence
            .get_latest_assistant_message(self.conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        {
            {
                let mut guard = self.ui_assistant_message_id.write().await;
                *guard = Some(message.id);
            }
            if let Some(steps) = message.steps {
                let mut guard = self.ui_steps.lock().await;
                *guard = steps;
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
            TaskProgressPayload::Text(p) => {
                info!(
                    target: "task::event",
                    seq,
                    task_id = %self.task_id,
                    event = "Text",
                    iteration = p.iteration,
                    stream_id = %p.stream_id,
                    stream_done = p.stream_done,
                    len = p.text.len(),
                    ts = %p.timestamp,
                    "emit"
                );
            }
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
                    .map_err(|e| TaskExecutorError::ChannelError(e.to_string()))?;
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
            let mut guard = self.ui_steps.lock().await;
            if let Some(step) = maybe_step.take() {
                guard.push(step);
            }
            guard.clone()
        };

        let status = status_override.unwrap_or("streaming");
        self.persist_ui_steps(&steps_snapshot, status).await
    }

    async fn persist_ui_steps(&self, steps: &[UiStep], status: &str) -> TaskExecutorResult<()> {
        let message_id = self
            .ui_persistence
            .upsert_assistant_message(self.conversation_id, steps, status)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut guard = self.ui_assistant_message_id.write().await;
            *guard = Some(message_id);
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

    async fn load_messages_from_snapshot(&self) -> TaskExecutorResult<()> {
        let stored_messages = self
            .agent_persistence
            .execution_messages()
            .list_by_execution(&self.task_id)
            .await
            .map_err(|e| TaskExecutorError::ContextRecoveryFailed(e.to_string()))?;

        let mut guard = self.messages.write().await;
        guard.clear();
        for stored in stored_messages {
            guard.push(LLMMessage {
                role: stored.role.as_str().to_string(),
                content: LLMMessageContent::Text(stored.content.clone()),
            });
        }
        Ok(())
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

fn estimate_message_tokens(content: &str) -> i64 {
    ((content.len() as f32) / 4.0).ceil() as i64
}

fn render_llm_message(content: &LLMMessageContent) -> String {
    match content {
        LLMMessageContent::Text(text) => text.clone(),
        LLMMessageContent::Parts(parts) => serde_json::to_string(parts).unwrap_or_default(),
    }
}

/// Parsed LLM response (thinking/tool_calls/final answer).
#[derive(Debug, Clone)]
pub struct LLMResponseParsed {
    pub thinking: Option<String>,
    pub tool_calls: Option<Vec<LLMToolCall>>,
    pub final_answer: Option<String>,
    pub raw_content: String,
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
