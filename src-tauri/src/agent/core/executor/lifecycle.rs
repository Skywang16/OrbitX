/*!
 * 任务生命周期管理
 */

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::task;
use tracing::{error, warn};
use uuid::Uuid;

use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::tools::{ToolResultContent, ToolResultStatus};
use crate::agent::types::{Block, ErrorBlock, TaskEvent, ToolBlock, ToolOutput, ToolStatus};
use crate::workspace::{WorkspaceService, UNGROUPED_WORKSPACE_PATH};

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
            .initialize_message_track(&params.user_prompt, params.images.as_deref())
            .await?;

        if ctx.checkpointing_enabled() {
            if let Err(err) = ctx.init_checkpoint(user_message_id).await {
                warn!("Failed to initialize checkpoint: {}", err);
            }
        }

        let (system_prompt, _) = self
            .prompt_orchestrator()
            .build_task_prompts(
                ctx.session_id,
                ctx.task_id.to_string(),
                &params.user_prompt,
                &ctx.cwd,
                &ctx.tool_registry(),
            )
            .await?;

        ctx.set_system_prompt(system_prompt).await?;

        // 自动检测会话是否有历史执行记录，有则恢复上下文
        let has_history = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_session(ctx.session_id, 2)
            .await
            .map(|execs| execs.len() > 1) // 当前执行 + 至少一个历史执行
            .unwrap_or(false);

        if has_history {
            self.restore_session_history(&ctx, ctx.session_id).await?;
        }

        ctx.add_user_message_with_images(params.user_prompt, params.images.as_deref())
            .await?;
        ctx.set_status(AgentTaskStatus::Running).await?;

        let executor = self.clone();
        let ctx_for_spawn = Arc::clone(&ctx);
        let model_id = params.model_id.clone();

        task::spawn(async move {
            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Task execution failed: {}", e);
            }
        });

        Ok(ctx)
    }

    async fn run_task_loop(
        &self,
        ctx: Arc<TaskContext>,
        model_id: String,
    ) -> TaskExecutorResult<()> {
        const MAX_SYNTAX_REPAIR_ROUNDS: usize = 2;

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
                        ctx.emit_event(TaskEvent::TaskCompleted {
                            task_id: ctx.task_id.to_string(),
                        })
                        .await?;
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
                        ctx.set_status(AgentTaskStatus::Error).await?;
                        let _ = ctx.fail_assistant_message(error_block.clone()).await;
                        let _ = ctx
                            .emit_event(TaskEvent::TaskError {
                                task_id: ctx.task_id.to_string(),
                                error: error_block,
                            })
                            .await;
                        break;
                    }

                    continue;
                }
                Err(e) => {
                    error!("Task failed: {}", e);
                    ctx.set_status(AgentTaskStatus::Error).await?;

                    let error_block = ErrorBlock {
                        code: "task.execution_error".to_string(),
                        message: e.to_string(),
                        details: None,
                    };

                    let _ = ctx.fail_assistant_message(error_block.clone()).await;
                    let _ = ctx
                        .emit_event(TaskEvent::TaskError {
                            task_id: ctx.task_id.to_string(),
                            error: error_block,
                        })
                        .await;
                    break;
                }
            }
        }

        // 任务结束后立刻从 active_tasks 移除，避免内存/确认状态泄漏
        self.active_tasks().remove(ctx.task_id.as_ref());

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
                name: "syntax_diagnostics".to_string(),
                status,
                input: tool_input,
                output: Some(ToolOutput {
                    content: serde_json::json!(preview.clone()),
                    cancel_reason: result.cancel_reason.clone(),
                    ext: result.ext_info.clone(),
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
        let _ = ctx
            .emit_event(TaskEvent::TaskCancelled {
                task_id: task_id.to_string(),
            })
            .await;

        self.active_tasks().remove(task_id);

        Ok(())
    }

    async fn restore_session_history(
        &self,
        ctx: &TaskContext,
        session_id: i64,
    ) -> TaskExecutorResult<()> {
        use crate::agent::persistence::MessageRole;
        use crate::llm::anthropic_types::{
            MessageContent, MessageParam, MessageRole as AnthropicRole,
        };

        let executions = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_session(session_id, 10)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if executions.is_empty() {
            return Ok(());
        }

        let current_task_id = ctx.task_id.to_string();
        let mut all_messages = Vec::new();

        for execution in executions.iter().rev() {
            if execution.execution_id == current_task_id {
                continue;
            }

            let messages = self
                .agent_persistence()
                .execution_messages()
                .list_by_execution(&execution.execution_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            if messages.is_empty() {
                continue;
            }

            for msg in messages {
                let role = match msg.role {
                    MessageRole::User => AnthropicRole::User,
                    MessageRole::Assistant => AnthropicRole::Assistant,
                    MessageRole::Tool | MessageRole::System => continue,
                };

                all_messages.push(MessageParam {
                    role,
                    content: MessageContent::Text(msg.content),
                });
            }
        }

        if !all_messages.is_empty() {
            ctx.restore_messages(all_messages).await?;
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
