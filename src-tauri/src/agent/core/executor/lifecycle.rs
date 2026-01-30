/*!
 * 任务生命周期管理
 */

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::task;
use tracing::{error, warn};
use uuid::Uuid;

use crate::agent::common::truncate_chars;
use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::tools::RunnableTool;
use crate::agent::tools::ToolAvailabilityContext;
use crate::agent::tools::{ToolResultContent, ToolResultStatus};
use crate::agent::types::{
    AgentSwitchBlock, Block, ErrorBlock, Message, MessageRole, MessageStatus, SubtaskBlock,
    SubtaskStatus, TaskEvent, ToolBlock, ToolOutput, ToolStatus,
};
use crate::workspace::{WorkspaceService, UNGROUPED_WORKSPACE_PATH};
use crate::{
    agent::common::llm_text::extract_text_from_llm_message, llm::service::LLMService,
};

struct RunTaskLoopDropGuard {
    executor: TaskExecutor,
    ctx: Arc<TaskContext>,
    armed: bool,
}

impl RunTaskLoopDropGuard {
    fn new(executor: TaskExecutor, ctx: Arc<TaskContext>) -> Self {
        Self {
            executor,
            ctx,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RunTaskLoopDropGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let ctx = Arc::clone(&self.ctx);
        let executor = self.executor.clone();
        ctx.abort();

        task::spawn(async move {
            // If we're being dropped mid-flight, make sure the UI isn't left with streaming blocks
            // and pending tools. Treat it as cancellation unless a terminal state was already set.
            let status = ctx.status().await;
            if matches!(
                status,
                AgentTaskStatus::Created | AgentTaskStatus::Running | AgentTaskStatus::Paused
            ) {
                let _ = ctx.set_status(AgentTaskStatus::Cancelled).await;
                let _ = ctx.cancel_assistant_message().await;
            }

            ctx.tool_registry()
                .cancel_pending_confirmations_for_task(&ctx, ctx.task_id.as_ref())
                .await;

            executor.active_tasks().remove(ctx.task_id.as_ref());
        });
    }
}

impl TaskExecutor {
    pub async fn execute_task(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Channel<TaskEvent>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        // 规范化参数：空工作区或 session_id=0 时使用未分组会话
        let params = self.normalize_task_params(params).await?;

        let ctx = self
            .build_or_restore_context(&params, Some(progress_channel))
            .await?;

        // 清空上次任务残留的 agent edit 集合，避免“诊断到旧文件”这种愚蠢行为。
        let _ = ctx.file_tracker().take_recent_agent_edits().await;

        ctx.emit_event(TaskEvent::TaskCreated {
            task_id: ctx.task_id.to_string(),
            session_id: ctx.session_id,
            workspace_path: ctx.cwd.to_string(),
        })
        .await?;

        // 创建 UI 消息（用户 + assistant 占位）
        let user_message_id = ctx
            .initialize_message_track(&params.user_prompt, params.images.as_deref(), false)
            .await?;

        // Persist model_id on the session so subtasks (Task tool) can inherit it reliably.
        // Otherwise older sessions created without model selection will fail with:
        // "No model_id set on session; cannot run subtask".
        let _ = ctx
            .agent_persistence()
            .sessions()
            .update_model_id(ctx.session_id, &params.model_id)
            .await;

        if ctx.checkpointing_enabled() {
            if let Err(err) = ctx.init_checkpoint(user_message_id).await {
                warn!("Failed to initialize checkpoint: {}", err);
            }
        }

        ctx.set_status(AgentTaskStatus::Running).await?;

        // Do not block UI on any network or heavy initialization.
        // Everything below runs in background after MessageCreated has been emitted.
        let executor = self.clone();
        let ctx_for_spawn = Arc::clone(&ctx);
        let model_id = params.model_id.clone();
        let user_prompt = params.user_prompt.clone();
        let images = params.images.clone();
        let system_reminders = params.system_reminders.clone();

        task::spawn(async move {
            // Initialize MCP tools for this workspace (network I/O), then build prompts using
            // the final tool registry (builtin + MCP) before starting the loop.
            let workspace_root = std::path::PathBuf::from(ctx_for_spawn.cwd.as_ref());
            let effective = match executor
                .settings_manager()
                .get_effective_settings(Some(workspace_root.clone()))
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to load effective settings: {}", e);
                    return;
                }
            };

            let workspace_settings = match executor
                .settings_manager()
                .get_workspace_settings(&workspace_root)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to load workspace settings: {}", e);
                    return;
                }
            };

            let _ = executor
                .mcp_registry()
                .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
                .await;

            // If prior subtasks were cancelled mid-flight, do NOT dump partial output into the
            // parent prompt. Backfill *real* summaries once per block using the LLM.
            if let Err(e) = executor
                .backfill_missing_subtask_summaries(&ctx_for_spawn, &model_id)
                .await
            {
                error!("Failed to backfill subtask summaries: {}", e);
            }

            // Restore history after backfilling summaries so the current turn's prompt sees them.
            let _ = ctx_for_spawn.reset_message_state().await;
            if let Err(e) = executor
                .restore_session_history(&ctx_for_spawn, ctx_for_spawn.session_id)
                .await
            {
                error!("Failed to restore session history: {}", e);
                return;
            }

            let availability_ctx = ToolAvailabilityContext {
                has_vector_index: executor.vector_search_engine().is_some(),
            };
            for tool in executor
                .mcp_registry()
                .get_tools_for_workspace(ctx_for_spawn.cwd.as_ref())
            {
                let name = tool.name().to_string();
                let _ = ctx_for_spawn
                    .tool_registry()
                    .register(
                        &name,
                        Arc::new(tool) as Arc<dyn RunnableTool>,
                        false,
                        &availability_ctx,
                    )
                    .await;
            }

            let (system_prompt, _) = match executor
                .prompt_orchestrator()
                .build_task_prompts(
                    ctx_for_spawn.session_id,
                    ctx_for_spawn.task_id.to_string(),
                    &user_prompt,
                    ctx_for_spawn.agent_type.as_ref(),
                    &ctx_for_spawn.cwd,
                    &ctx_for_spawn.tool_registry(),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to build task prompts: {}", e);
                    ctx_for_spawn.abort();
                    let _ = ctx_for_spawn.set_status(AgentTaskStatus::Error).await;

                    let error_block = ErrorBlock {
                        code: "task.prompt_build_failed".to_string(),
                        message: e.to_string(),
                        details: None,
                    };

                    let _ = ctx_for_spawn.fail_assistant_message(error_block.clone()).await;
                    if ctx_for_spawn.emits_task_events() {
                        let _ = ctx_for_spawn
                            .emit_event(TaskEvent::TaskError {
                                task_id: ctx_for_spawn.task_id.to_string(),
                                error: error_block,
                            })
                            .await;
                    }

                    executor.active_tasks().remove(ctx_for_spawn.task_id.as_ref());
                    return;
                }
            };

            let _ = ctx_for_spawn.set_system_prompt(system_prompt).await;
            let _ = ctx_for_spawn
                .add_user_message_with_reminders(user_prompt, images.as_deref(), &system_reminders)
                .await;

            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Task execution failed: {}", e);
            }
        });

        Ok(ctx)
    }

    pub(super) async fn run_task_loop(
        &self,
        ctx: Arc<TaskContext>,
        model_id: String,
    ) -> TaskExecutorResult<()> {
        const MAX_SYNTAX_REPAIR_ROUNDS: usize = 2;

        let mut drop_guard = RunTaskLoopDropGuard::new(self.clone(), Arc::clone(&ctx));
        let mut repair_round = 0usize;

        loop {
            // 直接调用ReactOrchestrator，传递self作为ReactHandler
            // 编译器会为TaskExecutor生成特化代码，完全内联
            let result = self
                .react_orchestrator()
                .run_react_loop(&ctx, &model_id, self)
                .await;

            match result {
                Ok(()) => {
                    let syntax_ok = self
                        .run_syntax_diagnostics_and_maybe_request_fix(&ctx, repair_round)
                        .await?;

                    if syntax_ok {
                        ctx.set_status(AgentTaskStatus::Completed).await?;
                        let context_usage = ctx.calculate_context_usage(&model_id).await;
                        ctx.finish_assistant_message(
                            crate::agent::types::MessageStatus::Completed,
                            None,
                            context_usage,
                        )
                        .await?;

                        if ctx.agent_type.as_ref() == "plan" {
                            let _ = self
                                .switch_session_agent_with_ctx(
                                    &ctx,
                                    "coder",
                                    Some("plan completed".to_string()),
                                )
                                .await;
                        }

                        if ctx.emits_task_events() {
                            ctx.emit_event(TaskEvent::TaskCompleted {
                                task_id: ctx.task_id.to_string(),
                            })
                            .await?;
                        }
                        break;
                    }

                    repair_round = repair_round.saturating_add(1);
                    if repair_round > MAX_SYNTAX_REPAIR_ROUNDS {
                        let error_block = ErrorBlock {
                            code: "task.syntax_diagnostics_failed".to_string(),
                            message: "Agent introduced syntax errors and failed to repair them"
                                .to_string(),
                            details: Some(
                                "syntax_diagnostics reported errors after max repair rounds"
                                    .to_string(),
                            ),
                        };

                        error!("Task failed: {}", error_block.message);
                        ctx.abort();
                        ctx.set_status(AgentTaskStatus::Error).await?;
                        let _ = ctx.fail_assistant_message(error_block.clone()).await;
                        if ctx.emits_task_events() {
                            let _ = ctx
                                .emit_event(TaskEvent::TaskError {
                                    task_id: ctx.task_id.to_string(),
                                    error: error_block,
                                })
                                .await;
                        }
                        break;
                    }

                    continue;
                }
                Err(e) => {
                    ctx.abort();
                    // Cancellation/interruption is not an "error". Treat it as a graceful stop so
                    // the UI doesn't see "Task execution interrupted" when the user cancels or
                    // when a new user message supersedes the current run.
                    if matches!(e, TaskExecutorError::TaskInterrupted) {
                        let status = ctx.status().await;
                        if !matches!(status, AgentTaskStatus::Cancelled) {
                            let _ = ctx.set_status(AgentTaskStatus::Cancelled).await;
                            let _ = ctx.cancel_assistant_message().await;
                            if ctx.emits_task_events() {
                                let _ = ctx
                                    .emit_event(TaskEvent::TaskCancelled {
                                        task_id: ctx.task_id.to_string(),
                                    })
                                    .await;
                            }
                        }
                        break;
                    }

                    error!("Task failed: {}", e);
                    ctx.set_status(AgentTaskStatus::Error).await?;

                    let error_block = ErrorBlock {
                        code: "task.execution_error".to_string(),
                        message: e.to_string(),
                        details: None,
                    };

                    let _ = ctx.fail_assistant_message(error_block.clone()).await;
                    if ctx.emits_task_events() {
                        let _ = ctx
                            .emit_event(TaskEvent::TaskError {
                                task_id: ctx.task_id.to_string(),
                                error: error_block,
                            })
                            .await;
                    }
                    break;
                }
            }
        }

        ctx.abort();
        ctx.tool_registry()
            .cancel_pending_confirmations_for_task(&ctx, ctx.task_id.as_ref())
            .await;

        // 任务结束后立刻从 active_tasks 移除，避免内存/确认状态泄漏
        self.active_tasks().remove(ctx.task_id.as_ref());
        drop_guard.disarm();

        Ok(())
    }

    async fn run_syntax_diagnostics_and_maybe_request_fix(
        &self,
        ctx: &TaskContext,
        repair_round: usize,
    ) -> TaskExecutorResult<bool> {
        let edited = ctx.file_tracker().take_recent_agent_edits().await;
        if edited.is_empty() {
            return Ok(true);
        }

        let abs_paths: Vec<String> = edited
            .into_iter()
            .map(|p| {
                std::path::PathBuf::from(ctx.cwd.as_ref())
                    .join(p)
                    .display()
                    .to_string()
            })
            .collect();

        let tool_args = serde_json::json!({ "paths": abs_paths });
        let tool_input = tool_args.clone();
        let tool_id = format!("syntax_diagnostics:{}", Uuid::new_v4());
        let started_at = chrono::Utc::now();

        ctx.assistant_append_block(Block::Tool(ToolBlock {
            id: tool_id.clone(),
            call_id: tool_id.clone(),
            name: "syntax_diagnostics".to_string(),
            status: ToolStatus::Running,
            input: tool_args.clone(),
            output: None,
            compacted_at: None,
            started_at,
            finished_at: None,
            duration_ms: None,
        }))
        .await?;

        let result = ctx
            .tool_registry()
            .execute_tool("syntax_diagnostics", ctx, tool_args)
            .await;

        let finished_at = chrono::Utc::now();
        let status = match result.status {
            ToolResultStatus::Success => ToolStatus::Completed,
            ToolResultStatus::Error => ToolStatus::Error,
            ToolResultStatus::Cancelled => ToolStatus::Cancelled,
        };

        let preview = tool_result_preview_text(&result);
        ctx.assistant_update_block(
            &tool_id,
            Block::Tool(ToolBlock {
                id: tool_id.clone(),
                call_id: tool_id.clone(),
                name: "syntax_diagnostics".to_string(),
                status,
                input: tool_input,
                output: Some(ToolOutput {
                    content: serde_json::json!(preview.clone()),
                    title: None,
                    metadata: result.ext_info.clone(),
                    cancel_reason: result.cancel_reason.clone(),
                }),
                compacted_at: None,
                started_at,
                finished_at: Some(finished_at),
                duration_ms: Some(
                    finished_at
                        .signed_duration_since(started_at)
                        .num_milliseconds()
                        .max(0),
                ),
            }),
        )
        .await?;

        let error_count = result
            .ext_info
            .as_ref()
            .and_then(|v| v.get("errorCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if error_count == 0 {
            return Ok(true);
        }

        ctx.add_user_message(format!(
            "The agent modified files but introduced syntax errors. Fix them and ensure syntax_diagnostics reports no errors.\nrepairRound={repair_round}\n{preview}"
        ))
        .await?;

        Ok(false)
    }

    pub async fn cancel_task(
        &self,
        task_id: &str,
        _reason: Option<String>,
    ) -> TaskExecutorResult<()> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        ctx.abort();
        ctx.set_status(AgentTaskStatus::Cancelled).await?;

        let _ = ctx.cancel_assistant_message().await;
        if ctx.emits_task_events() {
            let _ = ctx
                .emit_event(TaskEvent::TaskCancelled {
                    task_id: task_id.to_string(),
                })
                .await;
        }

        self.active_tasks().remove(task_id);

        Ok(())
    }

    pub(super) async fn restore_session_history(
        &self,
        ctx: &TaskContext,
        session_id: i64,
    ) -> TaskExecutorResult<()> {
        use crate::llm::anthropic_types::{
            MessageContent, MessageParam, MessageRole as AnthropicRole,
        };

        let stored = self
            .agent_persistence()
            .messages()
            .list_by_session(session_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if stored.is_empty() {
            return Ok(());
        }

        let mut restored = Vec::new();
        for msg in stored {
            let Some(text) = extract_prompt_text(&msg.blocks, &msg.role) else {
                continue;
            };

            let role = match msg.role {
                crate::agent::types::MessageRole::User => AnthropicRole::User,
                crate::agent::types::MessageRole::Assistant => AnthropicRole::Assistant,
            };

            restored.push(MessageParam {
                role,
                content: MessageContent::Text(text),
            });
        }

        if !restored.is_empty() {
            ctx.restore_messages(restored).await?;
        }

        Ok(())
    }

    /// 规范化任务参数：空工作区时自动使用未分组会话
    async fn normalize_task_params(
        &self,
        mut params: ExecuteTaskParams,
    ) -> TaskExecutorResult<ExecuteTaskParams> {
        let needs_ungrouped =
            params.workspace_path.is_empty() || params.workspace_path.trim().is_empty();

        if needs_ungrouped || params.session_id <= 0 {
            let workspace_path = if needs_ungrouped {
                UNGROUPED_WORKSPACE_PATH.to_string()
            } else {
                params.workspace_path.clone()
            };

            let service = WorkspaceService::new(self.database());
            let session = service
                .ensure_active_session(&workspace_path)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            params.workspace_path = workspace_path;
            params.session_id = session.id;
        }

        Ok(params)
    }

    async fn switch_session_agent_with_ctx(
        &self,
        ctx: &TaskContext,
        to_agent: &str,
        reason: Option<String>,
    ) -> TaskExecutorResult<()> {
        let from_agent = ctx.agent_type.as_ref().to_string();

        ctx.agent_persistence()
            .sessions()
            .update_agent_type(ctx.session_id, to_agent)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let mut message = ctx
            .agent_persistence()
            .messages()
            .create(
                ctx.session_id,
                MessageRole::Assistant,
                MessageStatus::Completed,
                vec![Block::AgentSwitch(AgentSwitchBlock {
                    from_agent,
                    to_agent: to_agent.to_string(),
                    reason,
                })],
                false,
                false,
                to_agent,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let now = chrono::Utc::now();
        message.finished_at = Some(now);
        message.duration_ms = Some(0);
        ctx.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        ctx.emit_event(TaskEvent::MessageCreated {
            task_id: ctx.task_id.to_string(),
            message: message.clone(),
        })
        .await?;

        ctx.emit_event(TaskEvent::MessageFinished {
            task_id: ctx.task_id.to_string(),
            message_id: message.id,
            status: MessageStatus::Completed,
            finished_at: now,
            duration_ms: 0,
            token_usage: None,
            context_usage: None,
        })
        .await?;

        Ok(())
    }
}

impl TaskExecutor {
    async fn backfill_missing_subtask_summaries(
        &self,
        ctx: &TaskContext,
        model_id: &str,
    ) -> TaskExecutorResult<()> {
        // Good taste: keep this bounded. Backfill a few per turn to avoid runaway cost.
        const MAX_BACKFILLS_PER_TURN: usize = 3;
        const MAX_TRANSCRIPT_CHARS: usize = 6000;
        const MAX_SUMMARY_CHARS: usize = 1200;

        let stored = ctx
            .agent_persistence()
            .messages()
            .list_by_session(ctx.session_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let mut targets: Vec<(Message, SubtaskBlock)> = Vec::new();
        for msg in stored {
            if !matches!(msg.role, MessageRole::Assistant) {
                continue;
            }
            for block in &msg.blocks {
                let Block::Subtask(b) = block else { continue };
                if b.summary.is_some() {
                    continue;
                }
                if !matches!(b.status, SubtaskStatus::Cancelled | SubtaskStatus::Error) {
                    continue;
                }
                targets.push((msg.clone(), b.clone()));
                if targets.len() >= MAX_BACKFILLS_PER_TURN {
                    break;
                }
            }
            if targets.len() >= MAX_BACKFILLS_PER_TURN {
                break;
            }
        }

        if targets.is_empty() {
            return Ok(());
        }

        let llm = LLMService::new(self.database());
        for (mut parent_msg, subtask) in targets {
            let child_messages = ctx
                .agent_persistence()
                .messages()
                .list_by_session(subtask.child_session_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            let transcript = build_subtask_transcript(&child_messages);
            let transcript = truncate_transcript(transcript, MAX_TRANSCRIPT_CHARS);
            if transcript.trim().is_empty() {
                continue;
            }

            let system = crate::llm::anthropic_types::SystemPrompt::Text(
                crate::agent::prompt::BuiltinPrompts::system_compaction().to_string(),
            );
            let prompt = crate::agent::prompt::BuiltinPrompts::system_subtask_summary_user()
                .replace("{{transcript}}", &transcript);

            let request = crate::llm::anthropic_types::CreateMessageRequest {
                model: model_id.to_string(),
                max_tokens: 512,
                system: Some(system),
                messages: vec![crate::llm::anthropic_types::MessageParam {
                    role: crate::llm::anthropic_types::MessageRole::User,
                    content: crate::llm::anthropic_types::MessageContent::Text(prompt),
                }],
                tools: None,
                stream: false,
                temperature: Some(0.2),
                top_p: None,
                top_k: None,
                metadata: None,
                stop_sequences: None,
            };

            let resp = llm
                .call(request)
                .await
                .map_err(|e| TaskExecutorError::LLMCallFailed(e.to_string()))?;
            let mut summary = extract_text_from_llm_message(&resp);
            summary = summary.trim().to_string();
            if summary.is_empty() {
                continue;
            }
            summary = truncate_chars(&summary, MAX_SUMMARY_CHARS);

            let Some(idx) = parent_msg
                .blocks
                .iter()
                .position(|b| matches!(b, Block::Subtask(s) if s.id == subtask.id))
            else {
                continue;
            };

            let mut updated = subtask.clone();
            updated.summary = Some(summary);
            parent_msg.blocks[idx] = Block::Subtask(updated.clone());

            ctx.agent_persistence()
                .messages()
                .update(&parent_msg)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            let _ = ctx
                .emit_event(TaskEvent::BlockUpdated {
                    task_id: ctx.task_id.to_string(),
                    message_id: parent_msg.id,
                    block_id: updated.id.clone(),
                    block: Block::Subtask(updated),
                })
                .await;
        }

        Ok(())
    }
}

fn build_subtask_transcript(messages: &[Message]) -> String {
    let mut out = Vec::new();
    for msg in messages {
        let role = match msg.role {
            MessageRole::User => "USER",
            MessageRole::Assistant => "ASSISTANT",
        };
        let text = extract_prompt_text(&msg.blocks, &msg.role).unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        out.push(format!("{role}:\n{text}"));
    }
    out.join("\n\n").trim().to_string()
}

fn truncate_transcript(transcript: String, max_chars: usize) -> String {
    if transcript.len() <= max_chars {
        return transcript;
    }
    // Keep the end (most recent actions) while preserving a small header.
    let head = transcript.chars().take(800).collect::<String>();
    let tail = transcript
        .chars()
        .rev()
        .take(max_chars.saturating_sub(head.len()).saturating_sub(50))
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{head}\n\n…\n\n{tail}").trim().to_string()
}

fn tool_result_preview_text(result: &crate::agent::tools::ToolResult) -> String {
    result
        .content
        .iter()
        .map(|c| match c {
            ToolResultContent::Success(s) | ToolResultContent::Error(s) => s.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_prompt_text(
    blocks: &[Block],
    role: &crate::agent::types::MessageRole,
) -> Option<String> {
    let mut parts = Vec::new();

    match role {
        crate::agent::types::MessageRole::User => {
            for block in blocks {
                if let Block::UserText(b) = block {
                    if !b.content.trim().is_empty() {
                        parts.push(b.content.trim().to_string());
                    }
                }
            }
        }
        crate::agent::types::MessageRole::Assistant => {
            for block in blocks {
                match block {
                    Block::Text(b) => {
                        if !b.content.trim().is_empty() {
                            parts.push(b.content.trim().to_string());
                        }
                    }
                    Block::Subtask(b) => {
                        if let Some(summary) = &b.summary {
                            if !summary.trim().is_empty() {
                                parts.push(summary.trim().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let out = parts.join("\n");
    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}
