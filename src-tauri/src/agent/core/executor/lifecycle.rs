/*!
 * Task lifecycle management
 */

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::task;
use tracing::{error, warn};
use uuid::Uuid;

use crate::agent::common::truncate_chars;
use crate::agent::core::context::AgentRunContext;
use crate::agent::core::executor::{AgentRunExecutor, ExecuteRunParams};
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::persistence::repositories::CreateMessageParams;
use crate::agent::tools::RunnableTool;
use crate::agent::tools::ToolAvailabilityContext;
use crate::agent::tools::{ToolResultContent, ToolResultStatus};
use crate::agent::types::{
    AgentRunEvent, AgentSwitchBlock, Block, ErrorBlock, MessageRole, MessageStatus, ToolBlock,
    ToolOutput, ToolStatus,
};
use crate::workspace::WorkspaceService;

struct RunTaskLoopDropGuard {
    executor: AgentRunExecutor,
    ctx: Arc<AgentRunContext>,
    armed: bool,
}

impl RunTaskLoopDropGuard {
    fn new(executor: AgentRunExecutor, ctx: Arc<AgentRunContext>) -> Self {
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
                if let Err(err) = ctx.set_status(AgentTaskStatus::Cancelled).await {
                    warn!("Failed to mark dropped task as cancelled: {}", err);
                }
                if let Err(err) = ctx.cancel_assistant_message().await {
                    warn!(
                        "Failed to cancel assistant message for dropped task: {}",
                        err
                    );
                }
            }

            ctx.tool_registry()
                .cancel_pending_confirmations_for_task(&ctx, ctx.run_id.as_ref())
                .await;

            executor.active_runs().remove(ctx.run_id.as_ref());
        });
    }
}

impl AgentRunExecutor {
    pub async fn execute_run(
        &self,
        params: ExecuteRunParams,
        progress_channel: Channel<AgentRunEvent>,
    ) -> AgentRunResult<Arc<AgentRunContext>> {
        // Normalize parameters: validate workspace is set and create session if needed
        let params = self.normalize_task_params(params).await?;

        let ctx = self
            .build_or_restore_context(&params, Some(progress_channel))
            .await?;

        // Clear the agent edit set from the previous task to avoid "diagnosing old files" behavior.
        ctx.file_tracker().take_recent_agent_edits().await;

        ctx.emit_event(AgentRunEvent::AgentRunCreated {
            run_id: ctx.run_id.to_string(),
            thread_id: ctx.thread_id,
            workspace_path: ctx.cwd.to_string(),
        })
        .await?;

        // Create UI message (user + assistant placeholder)
        let display_user_prompt = if let Some(cmd_id) = params.command_id.as_deref() {
            format!("<!-- command:{cmd_id} -->\n{}", params.user_prompt)
        } else {
            params.user_prompt.clone()
        };
        let user_message_id = ctx
            .initialize_message_track(&display_user_prompt, params.images.as_deref(), false)
            .await?;

        // Persist model_id on the session so subagents (Task tool) can inherit it reliably.
        // Otherwise older sessions created without model selection will fail with:
        // "No model_id set on session; cannot run subagent".
        if let Err(err) = ctx
            .agent_persistence()
            .threads()
            .update_model_id(ctx.thread_id, &params.model_id)
            .await
        {
            warn!("Failed to persist session model id: {}", err);
        }

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
        let llm_user_prompt = ctx.user_prompt.as_ref().to_string();
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

            if let Err(err) = executor
                .mcp_registry()
                .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
                .await
            {
                warn!("Failed to initialize MCP workspace servers: {}", err);
            }

            // Restore history after backfilling summaries so the current turn's prompt sees them.
            if let Err(err) = ctx_for_spawn.reset_message_state().await {
                warn!(
                    "Failed to reset message state before restoring history: {}",
                    err
                );
            }
            if let Err(e) = executor
                .restore_thread_history(
                    &ctx_for_spawn,
                    ctx_for_spawn.thread_id,
                    Some(user_message_id),
                )
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
                if let Err(err) = ctx_for_spawn
                    .tool_registry()
                    .register(
                        &name,
                        Arc::new(tool) as Arc<dyn RunnableTool>,
                        false,
                        &availability_ctx,
                    )
                    .await
                {
                    warn!("Failed to register MCP tool '{}': {}", name, err);
                }
            }

            let prompts = match executor
                .prompt_orchestrator()
                .build_task_prompts(
                    ctx_for_spawn.thread_id,
                    ctx_for_spawn.run_id.to_string(),
                    &llm_user_prompt,
                    ctx_for_spawn.agent_type.as_ref(),
                    &ctx_for_spawn.cwd,
                    &ctx_for_spawn.tool_registry(),
                    Some(&model_id),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to build task prompts: {}", e);
                    ctx_for_spawn.abort();
                    if let Err(err) = ctx_for_spawn.set_status(AgentTaskStatus::Error).await {
                        warn!(
                            "Failed to mark task as error after prompt build failure: {}",
                            err
                        );
                    }

                    let error_block = ErrorBlock {
                        code: "task.prompt_build_failed".to_string(),
                        message: e.to_string(),
                        details: None,
                    };

                    if let Err(err) = ctx_for_spawn
                        .fail_assistant_message(error_block.clone())
                        .await
                    {
                        warn!("Failed to persist assistant error message: {}", err);
                    }
                    if ctx_for_spawn.emits_task_events() {
                        if let Err(err) = ctx_for_spawn
                            .emit_event(AgentRunEvent::AgentRunError {
                                run_id: ctx_for_spawn.run_id.to_string(),
                                error: error_block,
                            })
                            .await
                        {
                            warn!("Failed to emit task error event: {}", err);
                        }
                    }

                    executor.active_runs().remove(ctx_for_spawn.run_id.as_ref());
                    return;
                }
            };

            if let Err(err) = ctx_for_spawn.set_system_prompt(prompts.instructions).await {
                error!("Failed to persist system prompt: {}", err);
                return;
            }
            if let Err(err) = ctx_for_spawn
                .set_developer_context(prompts.developer_context)
                .await
            {
                error!("Failed to persist developer context: {}", err);
                return;
            }
            if let Err(err) = ctx_for_spawn
                .add_user_message_with_reminders(
                    prompts.user_prompt,
                    images.as_deref(),
                    &system_reminders,
                )
                .await
            {
                error!("Failed to append user message with reminders: {}", err);
                return;
            }

            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Task execution failed: {}", e);
            }
        });

        Ok(ctx)
    }

    pub(super) async fn run_task_loop(
        &self,
        ctx: Arc<AgentRunContext>,
        model_id: String,
    ) -> AgentRunResult<()> {
        const MAX_SYNTAX_REPAIR_ROUNDS: usize = 2;

        let mut drop_guard = RunTaskLoopDropGuard::new(self.clone(), Arc::clone(&ctx));
        let mut repair_round = 0usize;

        loop {
            // Directly call ReactOrchestrator, passing self as ReactHandler
            // Compiler will generate specialized code for AgentRunExecutor, fully inlined
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
                            if let Err(err) = self
                                .switch_session_agent_with_ctx(
                                    &ctx,
                                    "coder",
                                    Some("plan completed".to_string()),
                                )
                                .await
                            {
                                warn!("Failed to switch completed plan session to coder: {}", err);
                            }
                        }

                        if ctx.emits_task_events() {
                            ctx.emit_event(AgentRunEvent::AgentRunCompleted {
                                run_id: ctx.run_id.to_string(),
                            })
                            .await?;
                        }

                        // Refresh session metadata (including title)
                        let ws_service = WorkspaceService::new(self.database());
                        if let Err(e) = ws_service.refresh_thread_title(ctx.thread_id).await {
                            warn!("Failed to refresh session title: {}", e);
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
                        if let Err(err) = ctx.fail_assistant_message(error_block.clone()).await {
                            warn!("Failed to persist assistant syntax error message: {}", err);
                        }
                        if ctx.emits_task_events() {
                            if let Err(err) = ctx
                                .emit_event(AgentRunEvent::AgentRunError {
                                    run_id: ctx.run_id.to_string(),
                                    error: error_block,
                                })
                                .await
                            {
                                warn!("Failed to emit syntax repair failure event: {}", err);
                            }
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
                    if matches!(e, AgentRunError::TaskInterrupted) {
                        let status = ctx.status().await;
                        if !matches!(status, AgentTaskStatus::Cancelled) {
                            if let Err(err) = ctx.set_status(AgentTaskStatus::Cancelled).await {
                                warn!("Failed to mark interrupted task as cancelled: {}", err);
                            }
                            if let Err(err) = ctx.cancel_assistant_message().await {
                                warn!(
                                    "Failed to cancel assistant message for interrupted task: {}",
                                    err
                                );
                            }
                            if ctx.emits_task_events() {
                                if let Err(err) = ctx
                                    .emit_event(AgentRunEvent::AgentRunCancelled {
                                        run_id: ctx.run_id.to_string(),
                                    })
                                    .await
                                {
                                    warn!("Failed to emit task cancelled event: {}", err);
                                }
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

                    if let Err(err) = ctx.fail_assistant_message(error_block.clone()).await {
                        warn!(
                            "Failed to persist assistant execution error message: {}",
                            err
                        );
                    }
                    if ctx.emits_task_events() {
                        if let Err(err) = ctx
                            .emit_event(AgentRunEvent::AgentRunError {
                                run_id: ctx.run_id.to_string(),
                                error: error_block,
                            })
                            .await
                        {
                            warn!("Failed to emit task execution error event: {}", err);
                        }
                    }
                    break;
                }
            }
        }

        ctx.abort();
        ctx.tool_registry()
            .cancel_pending_confirmations_for_task(&ctx, ctx.run_id.as_ref())
            .await;

        // Remove from active_tasks immediately after task completion to avoid memory/confirmation state leaks
        self.active_runs().remove(ctx.run_id.as_ref());
        drop_guard.disarm();

        Ok(())
    }

    async fn run_syntax_diagnostics_and_maybe_request_fix(
        &self,
        ctx: &AgentRunContext,
        repair_round: usize,
    ) -> AgentRunResult<bool> {
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
            .ok_or_else(|| {
                AgentRunError::InternalError(
                    "syntax_diagnostics result missing numeric errorCount".to_string(),
                )
            })?;

        if error_count == 0 {
            return Ok(true);
        }

        ctx.add_user_message(format!(
            "The agent modified files but introduced syntax errors. Fix them and ensure syntax_diagnostics reports no errors.\nrepairRound={repair_round}\n{preview}"
        ))
        .await?;

        Ok(false)
    }

    pub async fn cancel_run(&self, run_id: &str, _reason: Option<String>) -> AgentRunResult<()> {
        let ctx = self
            .active_runs()
            .get(run_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| AgentRunError::TaskNotFound(run_id.to_string()))?;

        ctx.abort();
        ctx.set_status(AgentTaskStatus::Cancelled).await?;

        if let Err(err) = ctx.cancel_assistant_message().await {
            warn!(
                "Failed to cancel assistant message during explicit cancel: {}",
                err
            );
        }
        if ctx.emits_task_events() {
            if let Err(err) = ctx
                .emit_event(AgentRunEvent::AgentRunCancelled {
                    run_id: run_id.to_string(),
                })
                .await
            {
                warn!("Failed to emit task cancelled event: {}", err);
            }
        }

        self.active_runs().remove(run_id);

        Ok(())
    }

    pub(super) async fn restore_thread_history(
        &self,
        ctx: &AgentRunContext,
        thread_id: i64,
        _exclude_message_id: Option<i64>,
    ) -> AgentRunResult<()> {
        use crate::agent::compaction::ThreadMessageLoader;

        let loader = ThreadMessageLoader::new(self.agent_persistence());
        let restored = loader
            .load_for_llm(thread_id)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        if !restored.is_empty() {
            ctx.restore_messages(restored).await?;
        }

        Ok(())
    }

    /// Normalize task parameters:
    /// - Validate workspace_path is not empty (required)
    /// - Reuse the workspace active thread when thread_id = 0
    /// - Create a new thread only when the workspace has no active thread yet
    async fn normalize_task_params(
        &self,
        mut params: ExecuteRunParams,
    ) -> AgentRunResult<ExecuteRunParams> {
        // Workspace path is now required
        if params.workspace_path.is_empty() || params.workspace_path.trim().is_empty() {
            return Err(AgentRunError::ConfigurationError(
                "workspace_path is required. Please open a workspace folder first.".to_string(),
            ));
        }

        let service = WorkspaceService::new(self.database());
        let title_source = params.user_prompt.clone();
        let title = truncate_chars(&title_source, 100);

        // Ensure workspace exists in database
        service
            .get_or_create_workspace(&params.workspace_path)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        if params.thread_id <= 0 {
            let thread = service
                .ensure_active_thread_with_title(&params.workspace_path, &title)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

            service
                .set_active_thread(&params.workspace_path, Some(thread.id))
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

            params.thread_id = thread.id;
        } else if !title.trim().is_empty() {
            // For an existing thread, eagerly write the user prompt as the title if it's still
            // empty. This ensures the sidebar title updates immediately when the frontend
            // processes agent_run_created (which triggers a selectThreadById / listThreadViews),
            // rather than waiting for the full agent_run_completed event.
            if let Ok(Some(thread)) = service.get_thread(params.thread_id).await {
                if thread.title.trim().is_empty() {
                    if let Err(e) = service.update_thread_title(thread.id, &title).await {
                        warn!("Failed to eagerly set thread title before run: {}", e);
                    }
                }
            }
        }

        Ok(params)
    }

    async fn switch_session_agent_with_ctx(
        &self,
        ctx: &AgentRunContext,
        to_agent: &str,
        reason: Option<String>,
    ) -> AgentRunResult<()> {
        let from_agent = ctx.agent_type.as_ref().to_string();

        ctx.agent_persistence()
            .threads()
            .update_agent_type(ctx.thread_id, to_agent)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let mut message = ctx
            .agent_persistence()
            .messages()
            .create(CreateMessageParams {
                thread_id: ctx.thread_id,
                role: MessageRole::Assistant,
                status: MessageStatus::Completed,
                blocks: vec![Block::AgentSwitch(AgentSwitchBlock {
                    from_agent,
                    to_agent: to_agent.to_string(),
                    reason,
                })],
                is_summary: false,
                is_internal: false,
                agent_type: to_agent,
                parent_message_id: None,
                model_id: None,
                provider_id: None,
            })
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let now = chrono::Utc::now();
        message.finished_at = Some(now);
        message.duration_ms = Some(0);
        ctx.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        ctx.emit_event(AgentRunEvent::MessageCreated {
            run_id: ctx.run_id.to_string(),
            message: message.clone(),
        })
        .await?;

        ctx.emit_event(AgentRunEvent::MessageFinished {
            run_id: ctx.run_id.to_string(),
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
