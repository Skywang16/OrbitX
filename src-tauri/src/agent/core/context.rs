use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::core::chain::Chain;
use crate::agent::events::TaskProgressPayload;
use crate::agent::persistence::prelude::{AgentTask, AgentTaskStatus, RepositoryManager};
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::state::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::state::manager::{
    StateEventEmitter, StateManager, TaskState, TaskStatus, TaskThresholds,
};
use crate::agent::types::{PlannedTask, TaskDetail};
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart, LLMToolCall};

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
    progress_channel: Arc<RwLock<Option<Channel<TaskProgressPayload>>>>,

    task_record: Arc<RwLock<AgentTask>>,
    messages: Arc<RwLock<Vec<LLMMessage>>>,
    tool_results: Arc<RwLock<Vec<ToolCallResult>>>,

    chain: Arc<RwLock<Chain>>,
    conversation: Arc<RwLock<Vec<String>>>,
    current_node_id: Arc<RwLock<Option<String>>>,
    task_detail: Arc<RwLock<Option<TaskDetail>>>,
    planned_tree: Arc<RwLock<Option<PlannedTask>>>,

    cancellation: CancellationToken,
    step_tokens: Arc<Mutex<Vec<CancellationToken>>>,
    pause_status: Arc<AtomicU8>, // 0: running, 1: paused, 2: pause & abort current step

    react_runtime: Arc<RwLock<ReactRuntime>>,
    state_manager: Arc<RwLock<StateManager>>,

    root_task_id: Arc<RwLock<Option<String>>>,
    parent_task_id: Arc<RwLock<Option<String>>>,
    children: Arc<RwLock<Vec<String>>>,
}

impl TaskContext {
    /// Construct a fresh context for a new task.
    pub async fn new(
        task: AgentTask,
        config: TaskExecutionConfig,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        repositories: Arc<RepositoryManager>,
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

        let task_id = task.task_id.clone();
        let conversation_id = task.conversation_id;
        let user_prompt = task.user_prompt.clone();
        let task_status = task.status.clone();
        let current_iteration = task.current_iteration;
        let error_count = task.error_count;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        Ok(Self {
            task_id,
            conversation_id,
            user_prompt: user_prompt.clone(),
            config,
            repositories,
            progress_channel: Arc::new(RwLock::new(progress_channel)),
            task_record: Arc::new(RwLock::new(task)),
            messages: Arc::new(RwLock::new(Vec::new())),
            tool_results: Arc::new(RwLock::new(Vec::new())),
            chain: Arc::new(RwLock::new(Chain::new(user_prompt))),
            conversation: Arc::new(RwLock::new(Vec::new())),
            current_node_id: Arc::new(RwLock::new(None)),
            task_detail: Arc::new(RwLock::new(None)),
            planned_tree: Arc::new(RwLock::new(None)),
            cancellation: CancellationToken::new(),
            step_tokens: Arc::new(Mutex::new(Vec::new())),
            pause_status: Arc::new(AtomicU8::new(0)),
            react_runtime: Arc::new(RwLock::new(ReactRuntime::new(runtime_config))),
            state_manager: Arc::new(RwLock::new(StateManager::new(
                task_state,
                StateEventEmitter::new(),
            ))),
            root_task_id: Arc::new(RwLock::new(None)),
            parent_task_id: Arc::new(RwLock::new(None)),
            children: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Restore a context from persistence (task + latest snapshot).
    pub async fn restore(
        task_id: String,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        repositories: Arc<RepositoryManager>,
    ) -> TaskExecutorResult<Self> {
        let task = repositories
            .agent_tasks()
            .find_by_task_id(&task_id)
            .await
            .map_err(|e| TaskExecutorError::ContextRecoveryFailed(e.to_string()))?
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.clone()))?;

        let config = task
            .config_json
            .as_ref()
            .and_then(|json| serde_json::from_str::<TaskExecutionConfig>(json).ok())
            .unwrap_or_default();

        let context = Self::new(task, config, progress_channel, repositories).await?;
        context.load_messages_from_snapshot().await?;
        Ok(context)
    }

    /// Attach a new progress channel (used when resuming tasks).
    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskProgressPayload>>) {
        let mut guard = self.progress_channel.write().await;
        *guard = channel;
    }

    /// Current status of the task.
    pub async fn status(&self) -> AgentTaskStatus {
        self.task_record.read().await.status.clone()
    }

    /// Set the task status and persist the change.
    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        let mut task = self.task_record.write().await;
        task.status = status.clone();
        self.repositories
            .agent_tasks()
            .update_status(
                &task.task_id,
                status.clone(),
                task.current_iteration,
                task.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut manager = self.state_manager.write().await;
            manager.update_task_status(map_status(&status), None);
        }
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> TaskExecutorResult<u32> {
        let mut task = self.task_record.write().await;
        task.current_iteration = task.current_iteration.saturating_add(1);
        let current = task.current_iteration;
        self.repositories
            .agent_tasks()
            .update_status(
                &task.task_id,
                task.status.clone(),
                task.current_iteration,
                task.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.state_manager.write().await.increment_iteration();
        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.task_record.read().await.current_iteration
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let mut task = self.task_record.write().await;
        task.error_count = task.error_count.saturating_add(1);
        let count = task.error_count;
        self.repositories
            .agent_tasks()
            .update_status(
                &task.task_id,
                task.status.clone(),
                task.current_iteration,
                task.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.write().await.increment_error_count();
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> TaskExecutorResult<()> {
        let mut task = self.task_record.write().await;
        task.error_count = 0;
        self.repositories
            .agent_tasks()
            .update_status(
                &task.task_id,
                task.status.clone(),
                task.current_iteration,
                task.error_count,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.write().await.reset_error_count();
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let task = self.task_record.read().await;
        if matches!(
            task.status,
            AgentTaskStatus::Cancelled | AgentTaskStatus::Completed | AgentTaskStatus::Error
        ) {
            return true;
        }
        let manager = self.state_manager.read().await;
        manager.should_halt()
            || task.current_iteration >= self.config.max_iterations
            || task.error_count >= self.config.max_errors
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
        let mut messages = self.messages.write().await;
        let mut parts: Vec<LLMMessagePart> = Vec::new();

        if let Some(thinking) = parsed.thinking {
            parts.push(LLMMessagePart::Text { text: thinking });
        }

        if let Some(tool_calls) = parsed.tool_calls {
            for call in tool_calls {
                parts.push(LLMMessagePart::ToolCall {
                    tool_call_id: call.id,
                    tool_name: call.name,
                    args: call.arguments,
                });
            }
        }

        if let Some(final_answer) = parsed.final_answer {
            parts.push(LLMMessagePart::Text { text: final_answer });
        }

        let content = if parts.is_empty() {
            LLMMessageContent::Text(parsed.raw_content)
        } else {
            LLMMessageContent::Parts(parts)
        };

        messages.push(LLMMessage {
            role: "assistant".to_string(),
            content,
        });
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
                tool_call_id: result.call_id,
                tool_name: result.tool_name,
                result: result.result,
            }]),
        });
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
            content: LLMMessageContent::Text(system_prompt),
        });
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text(user_prompt),
        });
        Ok(())
    }

    /// Snapshot messages for LLM request construction.
    pub async fn get_messages(&self) -> Vec<LLMMessage> {
        self.messages.read().await.clone()
    }

    /// Append a user message to the conversation history (without resetting prompts).
    pub async fn push_user_message(&self, text: String) {
        let mut messages = self.messages.write().await;
        messages.push(LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text(text),
        });
    }

    /// Append a system message to the conversation history (without resetting prompts).
    pub async fn push_system_message(&self, text: String) {
        let mut messages = self.messages.write().await;
        messages.push(LLMMessage {
            role: "system".to_string(),
            content: LLMMessageContent::Text(text),
        });
    }

    /// Persist latest message state to the repository snapshot table.
    pub async fn save_context_snapshot(&self) -> TaskExecutorResult<()> {
        let messages = self.messages.read().await;
        let snapshot = serde_json::to_value(&*messages)
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let iteration = self.task_record.read().await.current_iteration;
        self.repositories
            .agent_context_snapshots()
            .create_full_snapshot(&self.task_id, iteration, &snapshot, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    /// Send progress payload back to the frontend if a channel is attached.
    pub async fn send_progress(&self, payload: TaskProgressPayload) -> TaskExecutorResult<()> {
        let guard = self.progress_channel.read().await;
        if let Some(channel) = guard.as_ref() {
            channel
                .send(payload)
                .map_err(|e| TaskExecutorError::ChannelError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_messages_from_snapshot(&self) -> TaskExecutorResult<()> {
        if let Some(snapshot) = self
            .repositories
            .agent_context_snapshots()
            .get_latest_snapshot(&self.task_id)
            .await
            .map_err(|e| TaskExecutorError::ContextRecoveryFailed(e.to_string()))?
        {
            if !snapshot.messages_json.is_empty() {
                if let Ok(messages) =
                    serde_json::from_str::<Vec<LLMMessage>>(&snapshot.messages_json)
                {
                    let mut guard = self.messages.write().await;
                    *guard = messages;
                } else {
                    debug!(
                        "Failed to deserialize messages snapshot for task {}",
                        self.task_id
                    );
                }
            }
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
