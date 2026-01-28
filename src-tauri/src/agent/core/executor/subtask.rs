use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::common::truncate_chars;
use crate::agent::core::context::{SubtaskRequest, SubtaskResponse, TaskContext, TaskContextDeps};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::permissions::ToolFilter;
use crate::agent::tools::RunnableTool;
use crate::agent::types::{Block, MessageRole, SubtaskStatus};

use super::TaskExecutor;

pub async fn run_subtask(
    executor: &TaskExecutor,
    parent: &TaskContext,
    request: SubtaskRequest,
) -> TaskExecutorResult<SubtaskResponse> {
    let workspace_root = PathBuf::from(parent.cwd.as_ref());

    let parent_session = executor
        .agent_persistence()
        .sessions()
        .get(parent.session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            TaskExecutorError::StatePersistenceFailed(format!(
                "parent session {} not found",
                parent.session_id
            ))
        })?;

    let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
        .await
        .unwrap_or_default();
    let Some(sub_cfg) = agent_configs.get(request.subagent_type.as_str()) else {
        return Err(TaskExecutorError::ConfigurationError(format!(
            "Unknown subagent_type: {}",
            request.subagent_type
        )));
    };

    if !matches!(
        sub_cfg.mode,
        crate::agent::agents::config::AgentMode::Subagent
    ) {
        return Err(TaskExecutorError::ConfigurationError(format!(
            "Agent {} is not a subagent",
            sub_cfg.name
        )));
    }

    let child_session_id = match request.session_id {
        Some(id) => id,
        None => {
            let spawned_by = request.call_id.as_deref();
            let created = executor
                .agent_persistence()
                .sessions()
                .create(
                    &parent_session.workspace_path,
                    Some(&request.description),
                    &request.subagent_type,
                    Some(parent.session_id),
                    spawned_by,
                    parent_session.model_id.as_deref(),
                    parent_session.provider_id.as_deref(),
                )
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
            created.id
        }
    };

    let child_session = executor
        .agent_persistence()
        .sessions()
        .get(child_session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            TaskExecutorError::StatePersistenceFailed(format!(
                "child session {} not found",
                child_session_id
            ))
        })?;

    if child_session.parent_id != Some(parent.session_id) {
        return Err(TaskExecutorError::ConfigurationError(
            "Child session does not belong to parent".to_string(),
        ));
    }

    let model_id = child_session
        .model_id
        .clone()
        .or(parent_session.model_id.clone())
        .ok_or_else(|| {
            TaskExecutorError::ConfigurationError(
                "No model_id set on session; cannot run subtask".to_string(),
            )
        })?;

    let effective = executor
        .settings_manager()
        .get_effective_settings(Some(workspace_root.clone()))
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;

    let workspace_settings = executor
        .settings_manager()
        .get_workspace_settings(&workspace_root)
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;
    let _ = executor
        .mcp_registry()
        .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
        .await;

    let mcp_tools = executor
        .mcp_registry()
        .get_tools_for_workspace(parent.cwd.as_ref())
        .into_iter()
        .map(|t| Arc::new(t) as Arc<dyn RunnableTool>)
        .collect::<Vec<_>>();

    // Subagent safety: always block recursive task/todowrite to prevent infinite loops.
    let merged_tool_filter = sub_cfg
        .tool_filter
        .merge(&ToolFilter::blacklist(["task", "todowrite"]));

    let tool_registry = crate::agent::tools::create_tool_registry(
        "agent",
        effective.permissions,
        Some(merged_tool_filter),
        executor.tool_confirmations(),
        mcp_tools,
        executor.vector_search_engine(),
    )
    .await;

    let task_id = format!("task_{}", uuid::Uuid::new_v4());
    let cwd = parent.cwd.to_string();
    let progress_channel = parent.progress_channel().await;

    let ctx = TaskContext::new(
        task_id,
        child_session_id,
        request.prompt.clone(),
        request.subagent_type.clone(),
        crate::agent::config::TaskExecutionConfig::default(),
        cwd,
        false,
        // Mixed-view design: stream child agent tool/message events on the same channel, but
        // persist them to the child session. The UI merges sessions into one timeline.
        progress_channel,
        TaskContextDeps {
            tool_registry: Arc::clone(&tool_registry),
            repositories: executor.database(),
            agent_persistence: executor.agent_persistence(),
            checkpoint_service: executor.checkpoint_service(),
            workspace_changes: executor.workspace_changes(),
            subtask_runner: Arc::new(executor.clone()),
        },
    )
    .await?;

    let ctx = Arc::new(ctx);

    if let Some((checkpoint_id, workspace_root)) = parent.active_checkpoint_handle().await {
        ctx.inherit_checkpoint(checkpoint_id, workspace_root).await;
    }

    let parent_cancel = parent.create_stream_cancel_token();
    let ctx_for_cancel = Arc::clone(&ctx);
    tokio::spawn(async move {
        parent_cancel.cancelled().await;
        ctx_for_cancel.abort();
    });

    executor
        .restore_session_history(&ctx, child_session_id)
        .await?;

    let _ = ctx.file_tracker().take_recent_agent_edits().await;
    let _ = ctx.initialize_message_track(&request.prompt, None, true).await?;

    let (system_prompt, _) = executor
        .prompt_orchestrator()
        .build_task_prompts(
            ctx.session_id,
            ctx.task_id.to_string(),
            &request.prompt,
            ctx.agent_type.as_ref(),
            &ctx.cwd,
            &tool_registry,
        )
        .await?;
    ctx.set_system_prompt(system_prompt).await?;
    ctx.add_user_message(request.prompt).await?;
    ctx.set_status(crate::agent::core::status::AgentTaskStatus::Running)
        .await?;

    // Subtasks share the parent's event stream; they must also be reachable for tool confirmation
    // resolution (agent_tool_confirm looks up by task_id).
    executor
        .active_tasks()
        .insert(ctx.task_id.to_string(), Arc::clone(&ctx));

    let task_key = ctx.task_id.to_string();
    let run_result = executor.run_task_loop(Arc::clone(&ctx), model_id).await;
    executor.active_tasks().remove(&task_key);

    let (status, runtime_error) = match run_result {
        Ok(()) => (SubtaskStatus::Completed, None),
        Err(TaskExecutorError::TaskInterrupted) | Err(TaskExecutorError::TaskCancelled(_)) => {
            (SubtaskStatus::Cancelled, None)
        }
        Err(e) => (SubtaskStatus::Error, Some(e)),
    };

    let messages = executor
        .agent_persistence()
        .messages()
        .list_by_session(child_session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

    // Do NOT write partial output back into the parent context on cancellation.
    // Cancellation is an incomplete run; the parent task will backfill a real summary on next turn.
    let summary = match status {
        SubtaskStatus::Cancelled => None,
        SubtaskStatus::Error => runtime_error.map(|e| e.to_string()).or_else(|| {
            extract_last_assistant_text(&messages).map(|s| truncate_chars(&s, 1200))
        }),
        SubtaskStatus::Completed => extract_last_assistant_text(&messages)
            .map(|s| truncate_chars(&s, 2000))
            .or(Some("Subtask completed.".to_string())),
        SubtaskStatus::Pending | SubtaskStatus::Running => None,
    };

    Ok(SubtaskResponse {
        session_id: child_session_id,
        status,
        summary,
    })
}

fn extract_last_assistant_text(messages: &[crate::agent::types::Message]) -> Option<String> {
    for msg in messages.iter().rev() {
        if !matches!(msg.role, MessageRole::Assistant) {
            continue;
        }
        let mut parts = Vec::new();
        for block in &msg.blocks {
            match block {
                Block::Text(b) => {
                    let cleaned = clean_subtask_text(&b.content);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                Block::Error(b) => {
                    let cleaned = clean_subtask_text(&b.message);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                _ => {}
            }
        }
        let out = parts.join("\n").trim().to_string();
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn clean_subtask_text(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return false;
            }
            // Drop pseudo tool tags that some models emit as plain text.
            if t.starts_with('<') && t.ends_with('>') {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}

