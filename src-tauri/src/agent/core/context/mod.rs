pub mod chain;
pub mod states;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tokio::sync::Notify;
use tokio::sync::RwLock;

use self::chain::Chain;
use self::states::{ExecutionState, TaskStates};
use crate::agent::config::{AgentConfig, AgentRunConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::executor::ImageAttachment;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::persistence::repositories::CreateMessageParams;
use crate::agent::persistence::AgentPersistence;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::rollout::{RolloutItem, ThreadMeta};
use crate::agent::state::manager::{AgentRunStatus, StateManager, TaskState, TaskThresholds};
use crate::agent::state::session::ThreadContext;
use crate::agent::tools::ToolRegistry;
use crate::agent::types::{
    AgentRunEvent, Block, ErrorBlock, MessageRole as UiMessageRole, MessageStatus, SubagentStatus,
    TokenUsage, ToolStatus, UserImageBlock, UserTextBlock,
};
use crate::agent::workspace_changes::WorkspaceChangeJournal;
use crate::checkpoint::CheckpointService;
use crate::llm::anthropic_types::{
    ContentBlock, MessageContent, MessageParam, MessageRole as AnthropicRole, SystemPrompt,
};
use crate::settings::SettingsManager;
use crate::storage::DatabaseManager;
use tokio_util::sync::CancellationToken;
use tracing::warn;

#[async_trait::async_trait]
pub trait SubAgentRunner: Send + Sync {
    async fn run_subagent(
        &self,
        parent: &AgentRunContext,
        request: SubAgentRequest,
    ) -> AgentRunResult<SubAgentResponse>;

    async fn spawn_subagent(
        &self,
        parent: &AgentRunContext,
        request: SubAgentRequest,
        collab_call_id: String,
        collab_tool: String,
    ) -> AgentRunResult<i64>;

    async fn cancel_subagent(&self, parent: &AgentRunContext, thread_id: i64)
        -> AgentRunResult<()>;
}

#[derive(Debug, Clone)]
pub struct SubAgentRequest {
    pub description: String,
    pub prompt: String,
    pub profile: String,
    pub thread_id: Option<i64>,
    pub call_id: Option<String>,
    pub resume_existing: bool,
    /// Optional model override for this delegated execution. When set, the backing session
    /// uses this model instead of inheriting the parent's model_id.
    pub model_id: Option<String>,
    /// When true, create an isolated git worktree for this child execution session.
    /// The child execution operates in its own branch, preventing file conflicts
    /// when multiple child executions run in parallel.
    pub use_worktree: bool,
}

#[derive(Debug, Clone)]
pub struct SubAgentResponse {
    pub thread_id: i64,
    pub status: SubagentStatus,
    pub summary: Option<String>,
}

pub struct AgentRunContextDeps {
    pub tool_registry: Arc<ToolRegistry>,
    pub repositories: Arc<DatabaseManager>,
    pub agent_persistence: Arc<AgentPersistence>,
    pub checkpoint_service: Option<Arc<CheckpointService>>,
    pub workspace_changes: Arc<WorkspaceChangeJournal>,
    pub subagent_runner: Arc<dyn SubAgentRunner>,
    pub settings_manager: Arc<SettingsManager>,
}

pub struct AgentRunContextInit {
    pub run_id: String,
    pub thread_id: i64,
    pub user_prompt: String,
    pub agent_type: String,
    pub config: AgentRunConfig,
    pub workspace_path: String,
    pub emit_task_events: bool,
    pub progress_channel: Option<Channel<AgentRunEvent>>,
    pub deps: AgentRunContextDeps,
}

pub struct AgentRunContext {
    pub run_id: Arc<str>,
    pub thread_id: i64,
    pub user_prompt: Arc<str>,
    pub agent_type: Arc<str>,
    pub cwd: Arc<str>,
    emit_task_events: bool,
    config: AgentRunConfig,

    session: Arc<ThreadContext>,
    tool_registry: Arc<ToolRegistry>,
    subagent_runner: Arc<dyn SubAgentRunner>,
    state_manager: Arc<StateManager>,
    checkpoint_service: Option<Arc<CheckpointService>>,
    active_checkpoint: Arc<RwLock<Option<ActiveCheckpoint>>>,
    workspace_changes: Arc<WorkspaceChangeJournal>,
    workspace_key: Arc<str>,
    settings_manager: Arc<SettingsManager>,

    pub(crate) states: TaskStates,

    pause_status: AtomicU8,
    pause_notify: Arc<Notify>,
}

impl AgentRunContext {
    /// Construct a fresh context for a new task.
    pub async fn new(init: AgentRunContextInit) -> AgentRunResult<Self> {
        let AgentRunContextInit {
            run_id,
            thread_id,
            user_prompt,
            agent_type,
            config,
            workspace_path,
            emit_task_events,
            progress_channel,
            deps,
        } = init;
        let agent_config = AgentConfig::default();
        let runtime_config = ReactRuntimeConfig {
            max_iterations: agent_config.max_react_num,
            max_consecutive_errors: agent_config.max_react_error_streak,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
        };

        let task_status = AgentTaskStatus::Created;
        let current_iteration = 0u32;
        let error_count = 0u32;

        let mut task_state = TaskState::new(run_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        let normalized_workspace = workspace_path;
        let workspace_root = PathBuf::from(&normalized_workspace);
        let workspace_key: Arc<str> = Arc::from(normalized_workspace.as_str());

        let session = Arc::new(ThreadContext::new(
            run_id.clone(),
            thread_id,
            workspace_root.clone(),
            user_prompt.clone(),
            config,
            Arc::clone(&deps.repositories),
            Arc::clone(&deps.agent_persistence),
        ));

        ensure_thread_rollout(
            deps.agent_persistence.as_ref(),
            thread_id,
            &workspace_root,
            &user_prompt,
            &agent_type,
        )
        .await?;

        let execution = ExecutionState::new(task_status);
        let react_runtime = ReactRuntime::new(runtime_config);

        let states = TaskStates::new(execution, react_runtime, progress_channel);

        Ok(Self {
            run_id: Arc::from(run_id.as_str()),
            thread_id,
            user_prompt: Arc::from(user_prompt.as_str()),
            agent_type: Arc::from(agent_type.as_str()),
            cwd: Arc::from(normalized_workspace.as_str()),
            emit_task_events,
            config,
            session,
            tool_registry: deps.tool_registry,
            subagent_runner: deps.subagent_runner,
            state_manager: Arc::new(StateManager::new(task_state)),
            checkpoint_service: deps.checkpoint_service,
            active_checkpoint: Arc::new(RwLock::new(None)),
            workspace_changes: deps.workspace_changes,
            workspace_key,
            settings_manager: deps.settings_manager,
            states,
            pause_status: AtomicU8::new(0),
            pause_notify: Arc::new(Notify::new()),
        })
    }

    pub fn emits_task_events(&self) -> bool {
        self.emit_task_events
    }

    pub async fn note_agent_write_intent(&self, path: &Path) {
        self.workspace_changes
            .begin_agent_write(Arc::clone(&self.workspace_key), path.to_path_buf())
            .await;
    }

    pub async fn note_agent_read_snapshot(&self, path: &Path, content: &str) {
        self.workspace_changes
            .update_snapshot_from_read(Arc::clone(&self.workspace_key), path.to_path_buf(), content)
            .await;
    }

    pub async fn progress_channel(&self) -> Option<Channel<AgentRunEvent>> {
        self.states.progress_channel.lock().await.clone()
    }

    pub async fn current_assistant_message_id(&self) -> Option<i64> {
        self.states
            .messages
            .lock()
            .await
            .assistant_message
            .as_ref()
            .map(|message| message.id)
    }

    pub fn checkpointing_enabled(&self) -> bool {
        self.checkpoint_service.is_some()
    }

    pub async fn init_checkpoint(&self, message_id: i64) -> AgentRunResult<()> {
        let service = match &self.checkpoint_service {
            Some(service) => Arc::clone(service),
            None => return Ok(()),
        };

        let checkpoint = service
            .create_empty(self.thread_id, message_id, Path::new(self.cwd.as_ref()))
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut guard = self.active_checkpoint.write().await;
            *guard = Some(ActiveCheckpoint {
                id: checkpoint.id,
                workspace_root: PathBuf::from(&checkpoint.workspace_path),
            });
        }

        Ok(())
    }

    pub async fn active_checkpoint_handle(&self) -> Option<(i64, PathBuf)> {
        self.active_checkpoint
            .read()
            .await
            .as_ref()
            .map(|cp| (cp.id, cp.workspace_root.clone()))
    }

    pub async fn inherit_checkpoint(&self, checkpoint_id: i64, workspace_root: PathBuf) {
        let mut guard = self.active_checkpoint.write().await;
        *guard = Some(ActiveCheckpoint {
            id: checkpoint_id,
            workspace_root,
        });
    }

    pub async fn snapshot_file_before_edit(&self, path: &Path) -> AgentRunResult<()> {
        let service = match &self.checkpoint_service {
            Some(service) => Arc::clone(service),
            None => return Ok(()),
        };

        let handle = { self.active_checkpoint.read().await.clone() };

        if let Some(checkpoint) = handle {
            service
                .snapshot_file_before_edit(checkpoint.id, path, &checkpoint.workspace_root)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
        }

        Ok(())
    }

    pub fn session(&self) -> Arc<ThreadContext> {
        Arc::clone(&self.session)
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        self.session.file_tracker()
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        self.session.agent_persistence()
    }

    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    pub fn settings_manager(&self) -> Arc<SettingsManager> {
        Arc::clone(&self.settings_manager)
    }

    pub fn subagent_runner(&self) -> &dyn SubAgentRunner {
        self.subagent_runner.as_ref()
    }

    pub async fn status(&self) -> AgentTaskStatus {
        self.states.execution.read().await.runtime_status
    }

    pub async fn set_status(&self, status: AgentTaskStatus) -> AgentRunResult<()> {
        let session_status = {
            let mut exec = self.states.execution.write().await;
            exec.runtime_status = status;
            match status {
                AgentTaskStatus::Created | AgentTaskStatus::Paused => "idle",
                AgentTaskStatus::Running => "running",
                AgentTaskStatus::Completed => "completed",
                AgentTaskStatus::Error => "error",
                AgentTaskStatus::Cancelled => "cancelled",
            }
        };

        self.agent_persistence()
            .threads()
            .update_status(self.thread_id, session_status)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        self.state_manager
            .update_task_status(map_status(&status), None)
            .await;
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> AgentRunResult<u32> {
        let current = {
            let mut exec = self.states.execution.write().await;
            exec.current_iteration = exec.current_iteration.saturating_add(1);
            exec.message_sequence = 0;
            exec.current_iteration
        };

        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.states.execution.read().await.current_iteration
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> AgentRunResult<u32> {
        let count = {
            let mut exec = self.states.execution.write().await;
            exec.error_count = exec.error_count.saturating_add(1);
            exec.error_count
        };
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> AgentRunResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.error_count = 0;
        };
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let (status, iteration, errors) = {
            let exec = self.states.execution.read().await;
            (
                exec.runtime_status,
                exec.current_iteration,
                exec.error_count,
            )
        };
        if matches!(
            status,
            AgentTaskStatus::Cancelled | AgentTaskStatus::Completed | AgentTaskStatus::Error
        ) {
            return true;
        }
        iteration >= self.config.max_iterations || errors >= self.config.max_errors
    }

    /// Access the execution configuration (zero-cost access).
    pub fn config(&self) -> &AgentRunConfig {
        &self.config
    }

    /// Access repositories (used by LLM/tool bridges).
    pub fn repositories(&self) -> Arc<DatabaseManager> {
        self.session.repositories()
    }

    /// Batch read state - reduce lock contention, acquire all needed data in one lock
    /// Performance optimization: avoid multiple read().await calls
    pub(crate) async fn batch_read_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExecutionState) -> R,
    {
        let exec = self.states.execution.read().await;
        f(&exec)
    }

    pub async fn with_chain_mut<T>(&self, f: impl FnOnce(&mut Chain) -> T) -> T {
        let mut chain = self.states.chain.write().await;
        f(&mut chain)
    }

    /// Asynchronously check if task is aborted (with pause waiting)
    pub async fn check_aborted_async(&self, no_check_pause: bool) -> AgentRunResult<()> {
        if self.states.aborted.load(Ordering::SeqCst) || self.states.abort_token.is_cancelled() {
            return Err(AgentRunError::TaskInterrupted);
        }
        if no_check_pause {
            return Ok(());
        }
        while self.pause_status.load(Ordering::SeqCst) != 0 {
            tokio::select! {
                _ = self.states.abort_token.cancelled() => {
                    return Err(AgentRunError::TaskInterrupted);
                }
                _ = self.pause_notify.notified() => {}
            }
        }
        Ok(())
    }

    /// Abort task execution
    /// Simplified version: only needs to set aborted flag
    pub fn abort(&self) {
        self.states.aborted.store(true, Ordering::SeqCst);
        self.states.abort_token.cancel();

        // Mark react runtime as aborted
        let react_runtime = Arc::clone(&self.states.react_runtime);
        tokio::spawn(async move {
            let mut react = react_runtime.write().await;
            react.mark_abort();
        });
    }

    /// Check if already aborted
    pub fn is_aborted(&self) -> bool {
        self.states.aborted.load(Ordering::SeqCst) || self.states.abort_token.is_cancelled()
    }

    /// Create cancellation token for LLM stream
    /// This token will be automatically cancelled when task is aborted
    pub fn create_stream_cancel_token(&self) -> CancellationToken {
        self.states.abort_token.child_token()
    }

    /// No-op: orchestrator reloads from DB each iteration.
    pub async fn add_assistant_message(
        &self,
        _text: Option<String>,
        _tool_calls: Option<Vec<ContentBlock>>,
    ) -> AgentRunResult<()> {
        Ok(())
    }

    /// No-op: tool results persisted via assistant_update_block; orchestrator reloads from DB.
    pub async fn add_tool_results(&self, _results: Vec<AgentToolCallResult>) -> AgentRunResult<()> {
        Ok(())
    }

    pub async fn get_messages(&self) -> Vec<MessageParam> {
        self.states.execution.read().await.messages_vec()
    }

    pub async fn get_developer_context(&self) -> Vec<String> {
        let exec = self.states.execution.read().await;
        let mut ctx = exec.developer_context.clone();
        ctx.extend(exec.per_turn_developer_context.iter().cloned());
        ctx
    }

    pub async fn set_per_turn_developer_context(&self, items: Vec<String>) -> AgentRunResult<()> {
        self.states
            .execution
            .write()
            .await
            .per_turn_developer_context = items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        Ok(())
    }

    pub async fn get_system_prompt(&self) -> Option<SystemPrompt> {
        let exec = self.states.execution.read().await;
        match (&exec.system_prompt, &exec.system_prompt_overlay) {
            (None, None) => None,
            (Some(base), None) => Some(base.clone()),
            (None, Some(overlay)) => Some(overlay.clone()),
            (Some(base), Some(overlay)) => Some(merge_system_prompts(base, overlay)),
        }
    }

    pub async fn add_user_message(&self, text: String) -> AgentRunResult<()> {
        self.add_user_message_with_images(text, None).await
    }

    pub async fn add_user_message_with_images(
        &self,
        text: String,
        images: Option<&[ImageAttachment]>,
    ) -> AgentRunResult<()> {
        let content = if let Some(imgs) = images {
            // Build content blocks containing images and text
            let mut blocks: Vec<ContentBlock> = imgs
                .iter()
                .filter_map(|img| {
                    // Extract base64 data from data URL
                    // Format: data:image/jpeg;base64,/9j/4AAQ...
                    let parts: Vec<&str> = img.data_url.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        Some(ContentBlock::Image {
                            source: crate::llm::anthropic_types::ImageSource::Base64 {
                                media_type: img.mime_type.clone(),
                                data: parts[1].to_string(),
                            },
                            cache_control: None,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Add text block
            if !text.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: text.clone(),
                    cache_control: None,
                });
            }

            MessageContent::Blocks(blocks)
        } else {
            MessageContent::Text(text.clone())
        };

        {
            let mut exec = self.states.execution.write().await;
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content,
            });
        }
        Ok(())
    }

    /// Add a user message with optional images and system reminders.
    /// System reminders are wrapped in <system-reminder> tags and prepended to the user message.
    /// They are only sent to the LLM, not persisted to UI messages.
    pub async fn add_user_message_with_reminders(
        &self,
        text: String,
        images: Option<&[ImageAttachment]>,
        system_reminders: &[String],
    ) -> AgentRunResult<()> {
        // Build the final text with system reminders prepended
        let final_text = if system_reminders.is_empty() {
            text
        } else {
            let reminder_block = system_reminders
                .iter()
                .map(|r| format!("<system-reminder>\n{}\n</system-reminder>", r.trim()))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("{reminder_block}\n\n{text}")
        };

        self.add_user_message_with_images(final_text, images).await
    }

    pub async fn reset_message_state(&self) -> AgentRunResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.messages.clear();
            exec.message_sequence = 0;
        }
        Ok(())
    }

    /// Set system prompt in memory only; do not persist system message to DB.
    pub async fn set_system_prompt(&self, prompt: String) -> AgentRunResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.system_prompt = Some(SystemPrompt::Text(prompt));
        exec.system_prompt_overlay = None;
        Ok(())
    }

    pub async fn set_developer_context(&self, items: Vec<String>) -> AgentRunResult<()> {
        self.states.execution.write().await.developer_context = items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        Ok(())
    }

    /// A transient overlay appended to the base system prompt (e.g. loop warnings).
    pub async fn set_system_prompt_overlay(
        &self,
        overlay: Option<SystemPrompt>,
    ) -> AgentRunResult<()> {
        self.states.execution.write().await.system_prompt_overlay = overlay;
        Ok(())
    }

    pub async fn restore_messages(&self, messages: Vec<MessageParam>) -> AgentRunResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.messages.clear();
        for msg in messages {
            exec.messages.push(msg);
        }
        // Do not modify runtime_status, keep current state
        Ok(())
    }

    pub async fn emit_event(&self, event: AgentRunEvent) -> AgentRunResult<()> {
        self.persist_rollout_item_for_event(&event).await?;
        let channel_guard = self.states.progress_channel.lock().await;
        match channel_guard.as_ref() {
            Some(channel) => {
                channel.send(event).map_err(AgentRunError::ChannelError)?;
            }
            None => {
                tracing::warn!(
                    run_id = %self.run_id,
                    "emit_event called but channel not initialized, event dropped"
                );
            }
        }
        Ok(())
    }

    async fn persist_rollout_item_for_event(&self, event: &AgentRunEvent) -> AgentRunResult<()> {
        let item = match event {
            AgentRunEvent::AgentRunCreated {
                run_id,
                thread_id,
                workspace_path,
            } => Some(RolloutItem::AgentRunCreated {
                run_id: run_id.clone(),
                thread_id: *thread_id,
                workspace_path: workspace_path.clone(),
            }),
            // Message lifecycle is already persisted by MessageRepository create/update.
            AgentRunEvent::MessageCreated { .. }
            | AgentRunEvent::BlockAppended { .. }
            | AgentRunEvent::BlockUpdated { .. }
            | AgentRunEvent::MessageFinished { .. } => None,
            AgentRunEvent::AgentRunCompleted { run_id } => Some(RolloutItem::AgentRunCompleted {
                run_id: run_id.clone(),
            }),
            AgentRunEvent::AgentRunError { run_id, error } => Some(RolloutItem::AgentRunError {
                run_id: run_id.clone(),
                error: error.clone(),
            }),
            AgentRunEvent::AgentRunCancelled { run_id } => Some(RolloutItem::AgentRunCancelled {
                run_id: run_id.clone(),
            }),
            AgentRunEvent::ToolConfirmationRequested { .. } => None,
            _ => None,
        };

        if let Some(item) = item {
            self.agent_persistence()
                .rollout_recorder()
                .append(self.thread_id, item)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
        }
        match event {
            AgentRunEvent::AgentRunCreated { .. } => {
                self.agent_persistence()
                    .threads()
                    .touch(self.thread_id, Some("idle"), None, None)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            }
            AgentRunEvent::MessageCreated { message, .. } => {
                let first_user_message = match message.role {
                    UiMessageRole::User => message.blocks.iter().find_map(|block| match block {
                        Block::UserText(text) => Some(text.content.as_str()),
                        _ => None,
                    }),
                    UiMessageRole::Assistant => None,
                };
                self.agent_persistence()
                    .threads()
                    .touch(self.thread_id, Some("running"), None, first_user_message)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            }
            AgentRunEvent::AgentRunCompleted { .. } => {
                self.agent_persistence()
                    .threads()
                    .touch(self.thread_id, Some("completed"), None, None)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            }
            AgentRunEvent::AgentRunCancelled { .. } => {
                self.agent_persistence()
                    .threads()
                    .touch(self.thread_id, Some("cancelled"), None, None)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            }
            AgentRunEvent::AgentRunError { .. } => {
                self.agent_persistence()
                    .threads()
                    .touch(self.thread_id, Some("error"), None, None)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn initialize_message_track(
        &self,
        user_prompt: &str,
        images: Option<&[ImageAttachment]>,
        internal_user_message: bool,
    ) -> AgentRunResult<i64> {
        let mut user_blocks = Vec::new();

        if let Some(images) = images {
            user_blocks.extend(map_user_image_blocks(images));
        }
        user_blocks.push(Block::UserText(UserTextBlock {
            content: user_prompt.to_string(),
        }));

        let user_message = self
            .agent_persistence()
            .messages()
            .create(CreateMessageParams {
                thread_id: self.thread_id,
                role: UiMessageRole::User,
                status: MessageStatus::Completed,
                blocks: user_blocks,
                is_summary: false,
                is_internal: internal_user_message,
                agent_type: self.agent_type.as_ref(),
                parent_message_id: None,
                model_id: None,
                provider_id: None,
            })
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        self.emit_event(AgentRunEvent::MessageCreated {
            run_id: self.run_id.to_string(),
            message: user_message.clone(),
        })
        .await?;

        let assistant_message = self
            .agent_persistence()
            .messages()
            .create(CreateMessageParams {
                thread_id: self.thread_id,
                role: UiMessageRole::Assistant,
                status: MessageStatus::Streaming,
                blocks: Vec::new(),
                is_summary: false,
                is_internal: false,
                agent_type: self.agent_type.as_ref(),
                parent_message_id: Some(user_message.id),
                model_id: None,
                provider_id: None,
            })
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut msg_state = self.states.messages.lock().await;
            msg_state.assistant_message = Some(assistant_message.clone());
        }

        self.emit_event(AgentRunEvent::MessageCreated {
            run_id: self.run_id.to_string(),
            message: assistant_message,
        })
        .await?;

        Ok(user_message.id)
    }

    pub async fn assistant_append_block(&self, block: Block) -> AgentRunResult<i64> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                AgentRunError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        message.blocks.push(block.clone());
        let message_id = message.id;

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(AgentRunEvent::BlockAppended {
            run_id: self.run_id.to_string(),
            message_id,
            block,
        })
        .await?;

        Ok(message_id)
    }

    pub async fn assistant_update_block(&self, block_id: &str, block: Block) -> AgentRunResult<()> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                AgentRunError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        let Some(index) = find_block_index(&message.blocks, block_id) else {
            return Err(AgentRunError::StatePersistenceFailed(format!(
                "block {block_id} not found for update"
            )));
        };

        message.blocks[index] = block.clone();
        let message_id = message.id;

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(AgentRunEvent::BlockUpdated {
            run_id: self.run_id.to_string(),
            message_id,
            block_id: block_id.to_string(),
            block,
        })
        .await
    }

    pub async fn assistant_upsert_block(&self, block: Block) -> AgentRunResult<()> {
        let block_id = match &block {
            Block::Thinking(b) => b.id.clone(),
            Block::Text(b) => b.id.clone(),
            Block::Tool(b) => b.id.clone(),
            _ => {
                return Err(AgentRunError::StatePersistenceFailed(
                    "block type does not support upsert".to_string(),
                ))
            }
        };

        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                AgentRunError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        let index_opt = find_block_index(&message.blocks, &block_id);

        if let Some(index) = index_opt {
            message.blocks[index] = block.clone();
            let message_id = message.id;
            self.agent_persistence()
                .messages()
                .update(&message)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            self.states.messages.lock().await.assistant_message = Some(message);
            return self
                .emit_event(AgentRunEvent::BlockUpdated {
                    run_id: self.run_id.to_string(),
                    message_id,
                    block_id,
                    block,
                })
                .await;
        }

        let all_messages = self
            .agent_persistence()
            .messages()
            .list_by_thread(self.thread_id)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        for mut msg in all_messages {
            if let Some(idx) = find_block_index(&msg.blocks, &block_id) {
                msg.blocks[idx] = block.clone();
                let msg_id = msg.id;
                self.agent_persistence()
                    .messages()
                    .update(&msg)
                    .await
                    .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
                return self
                    .emit_event(AgentRunEvent::BlockUpdated {
                        run_id: self.run_id.to_string(),
                        message_id: msg_id,
                        block_id,
                        block,
                    })
                    .await;
            }
        }

        message.blocks.push(block.clone());
        let message_id = message.id;
        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
        self.states.messages.lock().await.assistant_message = Some(message);
        self.emit_event(AgentRunEvent::BlockAppended {
            run_id: self.run_id.to_string(),
            message_id,
            block,
        })
        .await
    }

    /// Calculate current session's context usage
    pub async fn calculate_context_usage(
        &self,
        model_id: &str,
    ) -> Option<crate::agent::types::ContextUsage> {
        use crate::agent::utils::tokenizer::{count_message_param_tokens, count_text_tokens};

        // Get model's context window size
        let context_window = match model_id {
            // Claude models
            s if s.contains("claude-3-5-sonnet") => 200_000,
            s if s.contains("claude-3-5-haiku") => 200_000,
            s if s.contains("claude-3-opus") => 200_000,
            s if s.contains("claude-3-sonnet") => 200_000,
            s if s.contains("claude-3-haiku") => 200_000,
            // GPT models
            s if s.contains("gpt-4") => 128_000,
            s if s.contains("gpt-3.5-turbo") => 16_384,
            // Default value
            _ => 128_000,
        };

        let mut tokens_used = 0usize;

        let execution = self.states.execution.read().await;

        // Count system prompt
        if let Some(system_prompt) = &execution.system_prompt {
            match system_prompt {
                SystemPrompt::Text(t) => {
                    tokens_used += count_text_tokens(t);
                }
                SystemPrompt::Blocks(blocks) => {
                    for block in blocks {
                        tokens_used += count_text_tokens(&block.text);
                    }
                }
            }
        }

        // Count conversation history
        for msg in &execution.messages {
            tokens_used += count_message_param_tokens(msg);
        }

        // Ensure not exceeding context window
        let tokens_used = (tokens_used as u32).min(context_window);

        Some(crate::agent::types::ContextUsage {
            tokens_used,
            context_window,
        })
    }

    pub async fn finish_assistant_message(
        &self,
        status: MessageStatus,
        token_usage: Option<TokenUsage>,
        context_usage: Option<crate::agent::types::ContextUsage>,
    ) -> AgentRunResult<()> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                AgentRunError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        let finished_at = Utc::now();
        let duration_ms = finished_at
            .signed_duration_since(message.created_at)
            .num_milliseconds()
            .max(0);

        message.status = status.clone();
        message.finished_at = Some(finished_at);
        message.duration_ms = Some(duration_ms);
        message.token_usage = token_usage.clone();
        message.context_usage = context_usage.clone();

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let message_id = message.id;
        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(AgentRunEvent::MessageFinished {
            run_id: self.run_id.to_string(),
            message_id,
            status,
            finished_at,
            duration_ms,
            token_usage,
            context_usage,
        })
        .await
    }

    pub async fn fail_assistant_message(&self, error: ErrorBlock) -> AgentRunResult<()> {
        let Some(mut message) = self.states.messages.lock().await.assistant_message.clone() else {
            return Ok(());
        };

        let now = Utc::now();
        let mut changed_blocks: Vec<(String, Block)> = Vec::new();

        let appended = Block::Error(error);
        message.blocks.push(appended.clone());

        for block in &mut message.blocks {
            match block {
                Block::Thinking(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Thinking(b.clone())));
                    }
                }
                Block::Text(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Text(b.clone())));
                    }
                }
                Block::Tool(b) => {
                    if matches!(b.status, ToolStatus::Running | ToolStatus::Pending) {
                        b.status = ToolStatus::Error;
                        b.finished_at = Some(now);
                        b.duration_ms = Some(
                            now.signed_duration_since(b.started_at)
                                .num_milliseconds()
                                .max(0),
                        );
                        changed_blocks.push((b.id.clone(), Block::Tool(b.clone())));
                    }
                }
                _ => {}
            }
        }

        message.status = MessageStatus::Error;
        message.finished_at = Some(now);
        message.duration_ms = Some(
            now.signed_duration_since(message.created_at)
                .num_milliseconds()
                .max(0),
        );

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let message_id = message.id;
        self.states.messages.lock().await.assistant_message = Some(message.clone());

        self.emit_event(AgentRunEvent::BlockAppended {
            run_id: self.run_id.to_string(),
            message_id,
            block: appended,
        })
        .await?;

        for (block_id, block) in changed_blocks {
            if let Err(err) = self
                .emit_event(AgentRunEvent::BlockUpdated {
                    run_id: self.run_id.to_string(),
                    message_id,
                    block_id,
                    block,
                })
                .await
            {
                warn!(
                    "Failed to emit block update while failing assistant message: {}",
                    err
                );
            }
        }

        self.emit_event(AgentRunEvent::MessageFinished {
            run_id: self.run_id.to_string(),
            message_id,
            status: MessageStatus::Error,
            finished_at: now,
            duration_ms: message.duration_ms.unwrap_or_else(|| {
                now.signed_duration_since(message.created_at)
                    .num_milliseconds()
                    .max(0)
            }),
            token_usage: None,
            context_usage: None,
        })
        .await?;
        Ok(())
    }

    pub async fn cancel_assistant_message(&self) -> AgentRunResult<()> {
        let Some(mut message) = self.states.messages.lock().await.assistant_message.clone() else {
            return Ok(());
        };

        let now = Utc::now();
        let mut changed_blocks: Vec<(String, Block)> = Vec::new();

        for block in &mut message.blocks {
            match block {
                Block::Thinking(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Thinking(b.clone())));
                    }
                }
                Block::Text(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Text(b.clone())));
                    }
                }
                Block::Tool(b) => {
                    if matches!(b.status, ToolStatus::Running | ToolStatus::Pending) {
                        b.status = ToolStatus::Cancelled;
                        b.finished_at = Some(now);
                        b.duration_ms = Some(
                            now.signed_duration_since(b.started_at)
                                .num_milliseconds()
                                .max(0),
                        );
                        changed_blocks.push((b.id.clone(), Block::Tool(b.clone())));
                    }
                }
                _ => {}
            }
        }

        message.status = MessageStatus::Cancelled;
        message.finished_at = Some(now);
        message.duration_ms = Some(
            now.signed_duration_since(message.created_at)
                .num_milliseconds()
                .max(0),
        );

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message.clone());

        for (block_id, block) in changed_blocks {
            if let Err(err) = self
                .emit_event(AgentRunEvent::BlockUpdated {
                    run_id: self.run_id.to_string(),
                    message_id: message.id,
                    block_id,
                    block,
                })
                .await
            {
                warn!(
                    "Failed to emit block update while cancelling assistant message: {}",
                    err
                );
            }
        }

        self.emit_event(AgentRunEvent::MessageFinished {
            run_id: self.run_id.to_string(),
            message_id: message.id,
            status: MessageStatus::Cancelled,
            finished_at: now,
            duration_ms: message.duration_ms.unwrap_or_else(|| {
                now.signed_duration_since(message.created_at)
                    .num_milliseconds()
                    .max(0)
            }),
            token_usage: None,
            context_usage: None,
        })
        .await?;
        Ok(())
    }
}

async fn ensure_thread_rollout(
    persistence: &AgentPersistence,
    thread_id: i64,
    workspace_root: &Path,
    user_prompt: &str,
    agent_type: &str,
) -> AgentRunResult<()> {
    let title = truncate_title(user_prompt);
    let created_at = Utc::now();
    let meta = ThreadMeta {
        thread_id,
        workspace_path: workspace_root.to_string_lossy().to_string(),
        title: title.clone(),
        thread_type: "agent".to_string(),
        agent_type: agent_type.to_string(),
        parent_thread_id: None,
        spawned_by_tool_call_id: None,
        model_id: None,
        provider_id: None,
        created_at,
    };
    let rollout_path = persistence
        .rollout_recorder()
        .ensure_thread_rollout(&meta)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
    persistence
        .threads()
        .update_rollout_path(thread_id, &rollout_path.to_string_lossy())
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
    Ok(())
}

fn truncate_title(value: &str) -> String {
    value.trim().chars().take(100).collect()
}

#[derive(Clone)]
struct ActiveCheckpoint {
    id: i64,
    workspace_root: PathBuf,
}

fn map_status(status: &AgentTaskStatus) -> AgentRunStatus {
    match status {
        AgentTaskStatus::Created => AgentRunStatus::Init,
        AgentTaskStatus::Running => AgentRunStatus::Running,
        AgentTaskStatus::Paused => AgentRunStatus::Paused,
        AgentTaskStatus::Completed => AgentRunStatus::Done,
        AgentTaskStatus::Error => AgentRunStatus::Error,
        AgentTaskStatus::Cancelled => AgentRunStatus::Aborted,
    }
}

fn map_user_image_blocks(images: &[ImageAttachment]) -> Vec<Block> {
    images
        .iter()
        .enumerate()
        .map(|(index, attachment)| {
            let ext = match attachment.mime_type.split('/').nth(1) {
                Some(ext) if !ext.trim().is_empty() => ext,
                _ => "image",
            };
            Block::UserImage(UserImageBlock {
                data_url: attachment.data_url.clone(),
                mime_type: attachment.mime_type.clone(),
                file_name: Some(format!("image_{index}.{ext}")),
                file_size: Some(attachment.data_url.len() as i64),
            })
        })
        .collect()
}

fn merge_system_prompts(
    base: &crate::llm::anthropic_types::SystemPrompt,
    overlay: &crate::llm::anthropic_types::SystemPrompt,
) -> crate::llm::anthropic_types::SystemPrompt {
    use crate::llm::anthropic_types::{SystemBlock, SystemPrompt};

    match (base, overlay) {
        (SystemPrompt::Text(a), SystemPrompt::Text(b)) => {
            SystemPrompt::Text(format!("{}\n\n{}", a.trim_end(), b.trim_start()))
        }
        (SystemPrompt::Blocks(a), SystemPrompt::Blocks(b)) => {
            let mut out = a.clone();
            out.extend(b.iter().cloned());
            SystemPrompt::Blocks(out)
        }
        (SystemPrompt::Text(a), SystemPrompt::Blocks(b)) => {
            let mut out = vec![SystemBlock {
                block_type: "text".to_string(),
                text: a.clone(),
                cache_control: None,
            }];
            out.extend(b.iter().cloned());
            SystemPrompt::Blocks(out)
        }
        (SystemPrompt::Blocks(a), SystemPrompt::Text(b)) => {
            let mut out = a.clone();
            out.push(SystemBlock {
                block_type: "text".to_string(),
                text: b.clone(),
                cache_control: None,
            });
            SystemPrompt::Blocks(out)
        }
    }
}

fn find_block_index(blocks: &[Block], block_id: &str) -> Option<usize> {
    blocks.iter().position(|block| match block {
        Block::Thinking(b) => b.id == block_id,
        Block::Text(b) => b.id == block_id,
        Block::Tool(b) => b.id == block_id,
        _ => false,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallResult {
    pub call_id: String,
    pub tool_name: String,
    pub result: Value,
    pub status: crate::agent::tools::ToolResultStatus,
    pub execution_time_ms: u64,
}
