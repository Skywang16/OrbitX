pub mod chain;
pub mod states;

use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use self::chain::Chain;
use self::states::{ExecutionState, PlanningState, TaskStates};
use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::ring_buffer::MessageRingBuffer;
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
use crate::agent::tools::ToolRegistry;
use crate::agent::types::TaskDetail;
use crate::agent::ui::{AgentUiPersistence, UiStep};
use crate::agent::utils::tokenizer::count_text_tokens;
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

const MAX_MESSAGE_HISTORY: usize = 100;

pub struct TaskContext {
    pub task_id: Arc<str>,
    pub conversation_id: i64,
    pub user_prompt: Arc<str>,
    pub cwd: Arc<str>,
    config: TaskExecutionConfig,

    session: Arc<SessionContext>,
    tool_registry: Arc<ToolRegistry>,
    state_manager: Arc<StateManager>,

    pub(crate) states: TaskStates,

    pause_status: AtomicU8,
    event_seq: AtomicU64,
}

impl TaskContext {
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
        let task_status = AgentTaskStatus::from(record.status);
        let current_iteration = record.current_iteration as u32;
        let error_count = record.error_count as u32;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

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
            config,
            Arc::clone(&repositories),
            Arc::clone(&agent_persistence),
            Arc::clone(&ui_persistence),
        ));

        let execution = ExecutionState::new(record, task_status);
        let planning = PlanningState::new(user_prompt.clone());
        let react_runtime = ReactRuntime::new(runtime_config);

        let states = TaskStates::new(execution, planning, react_runtime, progress_channel);

        Ok(Self {
            task_id: Arc::from(task_id.as_str()),
            conversation_id,
            user_prompt: Arc::from(user_prompt.as_str()),
            cwd: Arc::from(cwd.as_str()),
            config,
            session,
            tool_registry,
            state_manager: Arc::new(StateManager::new(task_state, StateEventEmitter::new())),
            states,
            pause_status: AtomicU8::new(0),
            event_seq: AtomicU64::new(0),
        })
    }

    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskProgressPayload>>) {
        *self.states.progress_channel.lock().await = channel;
    }

    pub async fn begin_iteration(&self, iteration_num: u32) -> Arc<IterationContext> {
        let iter_ctx = Arc::new(IterationContext::new(iteration_num, self.session()));
        self.states.execution.write().await.current_iteration = Some(Arc::clone(&iter_ctx));
        iter_ctx
    }

    pub async fn end_iteration(&self) -> Option<IterationSnapshot> {
        let maybe_ctx = self.states.execution.write().await.current_iteration.take();
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
        self.states.execution.read().await.current_iteration.clone()
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

    pub async fn status(&self) -> AgentTaskStatus {
        self.states.execution.read().await.runtime_status
    }

    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        let (execution_status, current_iteration, error_count) = {
            let mut exec = self.states.execution.write().await;
            exec.runtime_status = status;
            exec.record.status = ExecutionStatus::from(&status);
            (
                exec.record.status,
                exec.record.current_iteration,
                exec.record.error_count,
            )
        };

        if matches!(
            status,
            AgentTaskStatus::Completed | AgentTaskStatus::Cancelled | AgentTaskStatus::Error
        ) {
            self.agent_persistence()
                .agent_executions()
                .mark_finished(&self.task_id, execution_status)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        } else {
            self.agent_persistence()
                .agent_executions()
                .update_status(
                    &self.task_id,
                    execution_status,
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
            let mut exec = self.states.execution.write().await;
            exec.record.current_iteration = exec.record.current_iteration.saturating_add(1);
            exec.message_sequence = 0;
            (
                exec.record.current_iteration as u32,
                exec.record.current_iteration,
                exec.record.status,
                exec.record.error_count,
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
        self.states.execution.read().await.record.current_iteration as u32
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let (count, status, iteration, errors) = {
            let mut exec = self.states.execution.write().await;
            exec.record.error_count = exec.record.error_count.saturating_add(1);
            (
                exec.record.error_count as u32,
                exec.record.status,
                exec.record.current_iteration,
                exec.record.error_count,
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
            let mut exec = self.states.execution.write().await;
            exec.record.error_count = 0;
            (exec.record.status, exec.record.current_iteration)
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
            let exec = self.states.execution.read().await;
            (
                exec.runtime_status,
                exec.record.current_iteration as u32,
                exec.record.error_count as u32,
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

    /// Access the execution configuration (零成本访问).
    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    /// Access repositories (used by LLM/tool bridges).
    pub fn repositories(&self) -> Arc<DatabaseManager> {
        self.session.repositories()
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        Arc::clone(&self.state_manager)
    }

    /// 批量读取状态 - 减少锁争用，一次锁获取所有需要的数据
    /// 性能优化：避免多次 read().await
    pub(crate) async fn batch_read_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExecutionState) -> R,
    {
        let exec = self.states.execution.read().await;
        f(&exec)
    }

    #[allow(dead_code)]
    pub(crate) async fn batch_update_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ExecutionState) -> R,
    {
        let mut exec = self.states.execution.write().await;
        f(&mut exec)
    }

    pub async fn with_chain<T>(&self, f: impl FnOnce(&Chain) -> T) -> T {
        let planning = self.states.planning.read().await;
        f(&planning.chain)
    }

    pub async fn with_chain_mut<T>(&self, f: impl FnOnce(&mut Chain) -> T) -> T {
        let mut planning = self.states.planning.write().await;
        f(&mut planning.chain)
    }

    /// Push a user intervention (manual conversation entry).
    pub async fn push_conversation_message(&self, message: String) {
        self.states
            .planning
            .write()
            .await
            .conversation
            .push(message);
    }

    pub async fn drain_conversation(&self) -> Vec<String> {
        let mut planning = self.states.planning.write().await;
        std::mem::take(&mut planning.conversation)
    }

    pub async fn set_task_detail(&self, task: Option<TaskDetail>) {
        self.states.planning.write().await.task_detail = task;
    }

    pub async fn task_detail(&self) -> Option<TaskDetail> {
        self.states.planning.read().await.task_detail.clone()
    }

    pub async fn attach_parent(&self, parent_task_id: String, root_task_id: Option<String>) {
        let mut planning = self.states.planning.write().await;
        planning.parent_task_id = Some(parent_task_id.clone());
        planning.root_task_id = Some(root_task_id.unwrap_or(parent_task_id));
    }

    pub async fn add_child(&self, child_task_id: String) {
        let mut planning = self.states.planning.write().await;
        if !planning.children.contains(&child_task_id) {
            planning.children.push(child_task_id);
        }
    }

    /// Read current node identifier.
    pub async fn current_node_id(&self) -> Option<String> {
        self.states.planning.read().await.current_node_id.clone()
    }

    pub async fn set_planning_node(&self, node_id: Option<String>) {
        self.states.planning.write().await.current_node_id = node_id;
    }

    pub async fn check_aborted(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self
            .states
            .cancellation
            .lock()
            .await
            .main_token
            .is_cancelled()
        {
            return Err(TaskExecutorError::TaskInterrupted.into());
        }
        if no_check_pause {
            return Ok(());
        }
        loop {
            if self
                .states
                .cancellation
                .lock()
                .await
                .main_token
                .is_cancelled()
            {
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

    /// 中止任务执行
    /// 使用 spawn_blocking 确保即使在同步上下文中也能正确取消
    pub fn abort(&self) {
        use std::sync::Arc;
        
        // 克隆必要的状态以在异步任务中使用
        let cancellation = Arc::clone(&self.states.cancellation);
        let react_runtime = Arc::clone(&self.states.react_runtime);
        
        // 在后台异步执行取消操作，避免阻塞
        tokio::spawn(async move {
            // 使用阻塞锁确保取消一定能执行
            let cancellation_guard = cancellation.lock().await;
            cancellation_guard.main_token.cancel();
            
            // 取消所有步骤 tokens
            for token in cancellation_guard.step_tokens.iter() {
                token.cancel();
            }
            drop(cancellation_guard);
            
            // 标记 react 运行时为中止状态
            let mut react = react_runtime.write().await;
            react.mark_abort();
        });
        
        // 同步部分：立即尝试中止当前步骤
        self.abort_current_steps();
    }

    pub async fn reset_cancellation(&self) {
        let mut cancellation = self.states.cancellation.lock().await;
        cancellation.main_token = CancellationToken::new();
        cancellation.step_tokens.clear();
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

    pub fn register_step_token(&self) -> CancellationToken {
        let token = if let Ok(cancellation) = self.states.cancellation.try_lock() {
            cancellation.main_token.child_token()
        } else {
            CancellationToken::new()
        };
        if let Ok(mut cancellation) = self.states.cancellation.try_lock() {
            cancellation.step_tokens.push(token.clone());
        }
        token
    }

    fn abort_current_steps(&self) {
        if let Ok(mut cancellation) = self.states.cancellation.try_lock() {
            for token in cancellation.step_tokens.drain(..) {
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
            let mut exec = self.states.execution.write().await;
            exec.messages.push(MessageParam {
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

        // Persist each tool result as its own Tool message entry
        for result in &results {
            if let Ok(serialized) = serde_json::to_string(result) {
                let _ = self
                    .append_message(MessageRole::Tool, &serialized, false)
                    .await;
            }
        }

        {
            let mut exec = self.states.execution.write().await;
            exec.tool_results.extend(results);
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Blocks(blocks),
            });
        }
    }

    // Deprecated in zero-abstraction model: initial prompts are handled explicitly by caller.
    // Retained signature temporarily, but now implemented using set_system_prompt + add_user_message semantics without DB writes for system.
    pub async fn set_initial_prompts(
        &self,
        system_prompt: String,
        user_prompt: String,
    ) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.system_prompt = Some(SystemPrompt::Text(system_prompt));
            exec.messages.clear();
            exec.message_sequence = 0;
        }
        self.add_user_message(user_prompt).await;
        Ok(())
    }

    pub async fn get_messages(&self) -> Vec<MessageParam> {
        self.states.execution.read().await.messages_vec()
    }

    pub async fn get_system_prompt(&self) -> Option<SystemPrompt> {
        self.states.execution.read().await.system_prompt.clone()
    }

    pub async fn with_messages<T>(
        &self,
        f: impl FnOnce(&MessageRingBuffer<MessageParam, MAX_MESSAGE_HISTORY>) -> T,
    ) -> T {
        let exec = self.states.execution.read().await;
        f(&exec.messages)
    }

    pub async fn add_user_message(&self, text: String) {
        {
            let mut exec = self.states.execution.write().await;
            let before = exec.messages.len();
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Text(text.clone()),
            });
            let after = exec.messages.len();
            info!(
                "[TaskContext] add_user_message: messages {} -> {}",
                before, after
            );
        }
        let _ = self.append_message(MessageRole::User, &text, false).await;
    }

    pub async fn reset_message_state(&self) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.messages.clear();
            exec.message_sequence = 0;
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
        self.states.execution.write().await.system_prompt = Some(SystemPrompt::Text(prompt));
        Ok(())
    }

    // Deprecated: system prompt is stored separately and not part of messages.
    pub async fn update_system_prompt(&self, new_system_prompt: String) -> TaskExecutorResult<()> {
        self.states.execution.write().await.system_prompt =
            Some(SystemPrompt::Text(new_system_prompt));
        Ok(())
    }

    pub async fn restore_messages(&self, messages: Vec<MessageParam>) -> TaskExecutorResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.messages.clear();
        for msg in messages {
            exec.messages.push(msg);
        }
        // 不修改 runtime_status，保持当前状态
        info!(
            "[TaskContext] restore_messages: Restored {} messages for task {}",
            exec.messages.len(),
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
            let mut exec = self.states.execution.write().await;
            let iteration = exec.record.current_iteration;
            let seq = exec.message_sequence;
            exec.message_sequence = seq.saturating_add(1);
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
            let mut exec = self.states.execution.write().await;
            exec.ui_assistant_message_id = Some(assistant_message_id);
        }
        self.states.ui.lock().await.steps.clear();
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
            let mut exec = self.states.execution.write().await;
            exec.ui_assistant_message_id = Some(assistant_message_id);
        }
        self.states.ui.lock().await.steps.clear();
        Ok(())
    }

    pub async fn restore_ui_track(&self) -> TaskExecutorResult<()> {
        if let Some(message) = self
            .ui_persistence()
            .get_latest_assistant_message(self.conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        {
            let mut exec = self.states.execution.write().await;
            exec.ui_assistant_message_id = Some(message.id);
            if let Some(steps) = message.steps {
                self.states.ui.lock().await.steps = steps;
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
            let channel_guard = self.states.progress_channel.lock().await;
            if let Some(channel) = channel_guard.as_ref() {
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
            let mut ui = self.states.ui.lock().await;
            if let Some(step) = maybe_step.take() {
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
                        existing.content.push_str(&step.content);
                        existing.timestamp = step.timestamp;
                        existing.metadata = step.metadata;
                    } else {
                        ui.steps.push(step);
                    }
                } else {
                    ui.steps.push(step);
                }
            }
            ui.steps.clone()
        };

        let status = status_override.unwrap_or("streaming");
        self.persist_ui_steps(&steps_snapshot, status).await
    }

    async fn persist_ui_steps(&self, steps: &[UiStep], status: &str) -> TaskExecutorResult<()> {
        let current_id = self.states.execution.read().await.ui_assistant_message_id;

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
