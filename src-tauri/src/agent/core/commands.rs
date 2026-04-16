/*!
 * AgentRunExecutor Tauri command interface
 */

use crate::agent::agents::AgentConfigLoader;
use crate::agent::command_system::{CommandConfigLoader, CommandRenderResult, CommandSummary};
use crate::agent::core::executor::{AgentRunExecutor, AgentRunSummary, ExecuteRunParams};
use crate::agent::persistence::repositories::CreateMessageParams;
use crate::agent::skill::SkillSummary;
use crate::agent::tools::registry::ToolConfirmationDecision;
use crate::agent::types::{AgentRunEvent, AgentSwitchBlock, Block, MessageRole, MessageStatus};
use crate::agent::workspace_changes::{ChangeKind, PendingChange, WorkspaceChangeJournal};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{ipc::Channel, State};

/// Agent run executor state management
pub struct AgentRunExecutorState {
    pub executor: Arc<AgentRunExecutor>,
}

impl AgentRunExecutorState {
    pub fn new(executor: Arc<AgentRunExecutor>) -> Self {
        Self { executor }
    }
}

/// Execute agent run
#[tauri::command]
pub async fn agent_execute_run(
    state: State<'_, AgentRunExecutorState>,
    changes: State<'_, Arc<WorkspaceChangeJournal>>,
    params: ExecuteRunParams,
    channel: Channel<AgentRunEvent>,
) -> TauriApiResult<EmptyData> {
    let mut params = params;

    // Collect workspace file changes as a system reminder (not persisted to UI messages)
    match std::path::PathBuf::from(&params.workspace_path).canonicalize() {
        Ok(workspace_root) => {
            let workspace_key: std::sync::Arc<str> =
                std::sync::Arc::from(workspace_root.to_string_lossy().to_string());
            let pending = changes.take_pending_by_key(workspace_key).await;
            let notice = build_file_change_notice(&pending);
            if !notice.is_empty() {
                params.system_reminders.push(notice);
            }
        }
        Err(err) => {
            tracing::warn!(
                "Failed to canonicalize workspace path '{}': {}",
                params.workspace_path,
                err
            );
        }
    }

    match state.executor.execute_run(params, channel).await {
        Ok(_context) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("❌ Agent run execution failed: {}", e);
            match e {
                crate::agent::error::AgentRunError::TooManyActiveTasksGlobal { .. } => {
                    Ok(api_error!("agent.too_many_active_tasks_global"))
                }
                _ => Ok(api_error!("agent.execute_failed")),
            }
        }
    }
}

fn build_file_change_notice(changes: &[PendingChange]) -> String {
    if changes.is_empty() {
        return String::new();
    }

    let now = crate::file_watcher::now_timestamp_ms();
    let mut latest: std::collections::BTreeMap<String, &PendingChange> =
        std::collections::BTreeMap::new();
    for change in changes {
        latest.insert(change.relative_path.clone(), change);
    }
    let total_files = latest.len();

    let mut lines = vec![
        "WORKSPACE FILE CHANGES (since your last message)".to_string(),
        "These changes were made by the user or external tools. Treat them as authoritative."
            .to_string(),
        "Do NOT revert them. If you need to edit any of these files, re-read first with read_file to avoid overwriting user changes.".to_string(),
        String::new(),
    ];

    const MAX_FILES: usize = 20;
    let mut shown = 0usize;
    for (path, change) in latest.iter() {
        if shown >= MAX_FILES {
            break;
        }
        shown += 1;

        let age_ms = now.saturating_sub(change.observed_at_ms);
        let age = format_age(age_ms);

        let kind = match change.kind {
            ChangeKind::Created => "created",
            ChangeKind::Modified => "modified",
            ChangeKind::Deleted => "deleted",
            ChangeKind::Renamed => "renamed",
        };

        if change.large_change || change.patch.is_none() {
            let suffix = if change.large_change {
                "Large change detected; re-read before editing."
            } else {
                "Changed; re-read before editing if needed."
            };
            lines.push(format!("- {path} ({kind}, {age} ago): {suffix}"));
            continue;
        }

        lines.push(format!("- {path} ({kind}, {age} ago):"));
        lines.push("```diff".to_string());
        let Some(patch) = change.patch.as_deref() else {
            tracing::warn!(
                path = %path,
                kind = %kind,
                "Workspace change summary missing patch for non-large change"
            );
            lines.pop();
            lines.push(format!(
                "- {path} ({kind}, {age} ago): Changed; patch unavailable, re-read before editing."
            ));
            continue;
        };
        lines.push(patch.to_string());
        lines.push("```".to_string());
    }

    let omitted = total_files.saturating_sub(shown);
    if omitted > 0 {
        lines.push(String::new());
        lines.push(format!(
            "(Plus {omitted} more changed files omitted to keep context small.)"
        ));
    }

    lines.join("\n").trim().to_string()
}

fn format_age(age_ms: u64) -> String {
    if age_ms < 1_000 {
        return "just now".to_string();
    }
    let secs = age_ms / 1_000;
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    format!("{days}d")
}

/// Cancel agent run
#[tauri::command]
pub async fn agent_cancel_run(
    state: State<'_, AgentRunExecutorState>,
    run_id: String,
    reason: Option<String>,
) -> TauriApiResult<EmptyData> {
    match state.executor.cancel_run(&run_id, reason).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("❌ Cancel agent run failed: {}", e);
            Ok(api_error!("agent.cancel_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmationParams {
    pub request_id: String,
    pub decision: ToolConfirmationDecision,
}

/// Return tool confirmation result
#[tauri::command]
pub async fn agent_tool_confirm(
    state: State<'_, AgentRunExecutorState>,
    params: ToolConfirmationParams,
) -> TauriApiResult<EmptyData> {
    tracing::info!(
        "🔐 [cmd] agent_tool_confirm called: request_id={} decision={:?}",
        params.request_id,
        params.decision
    );
    let run_id = state
        .executor
        .tool_confirmations()
        .lookup_task_id(&params.request_id);
    tracing::info!(
        "🔐 [cmd] lookup_task_id result: request_id={} -> run_id={:?}",
        params.request_id,
        run_id
    );
    let ctx = run_id.and_then(|run_id| {
        let found = state
            .executor
            .active_runs()
            .get(&run_id)
            .map(|entry| Arc::clone(entry.value()));
        tracing::info!(
            "🔐 [cmd] active_runs.get({}): found={}",
            run_id,
            found.is_some()
        );
        found
    });

    let ctx = match ctx {
        Some(ctx) => ctx,
        None => {
            tracing::warn!(
                "🔐 [cmd] confirm NOT FOUND: request_id={}, active_runs_count={}",
                params.request_id,
                state.executor.active_runs().len()
            );
            return Ok(api_error!("agent.tool_confirm_not_found"));
        }
    };

    let ok = ctx
        .tool_registry()
        .resolve_confirmation(&ctx, &params.request_id, params.decision)
        .await;

    tracing::info!(
        "🔐 [cmd] resolve_confirmation result: request_id={} ok={}",
        params.request_id,
        ok
    );

    if ok {
        Ok(api_success!())
    } else {
        Ok(api_error!("agent.tool_confirm_not_found"))
    }
}

/// List active agent runs
#[tauri::command]
pub async fn agent_list_runs(
    state: State<'_, AgentRunExecutorState>,
    thread_id: Option<i64>,
    status_filter: Option<String>,
) -> TauriApiResult<Vec<AgentRunSummary>> {
    match state.executor.list_runs(thread_id, status_filter).await {
        Ok(tasks) => Ok(api_success!(tasks)),
        Err(e) => {
            tracing::error!("❌ List agent runs failed: {}", e);
            Ok(api_error!("agent.list_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommandsParams {
    pub workspace_path: String,
}

/// List built-in commands
#[tauri::command]
pub async fn agent_list_commands(
    _state: State<'_, AgentRunExecutorState>,
    _params: ListCommandsParams,
) -> TauriApiResult<Vec<CommandSummary>> {
    let mut out = CommandConfigLoader::all()
        .values()
        .map(CommandConfigLoader::summarize)
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(api_success!(out))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderCommandParams {
    pub workspace_path: String,
    pub name: String,
    pub input: String,
}

/// Render built-in command template (only does `{{input}}` replacement)
#[tauri::command]
pub async fn agent_render_command(
    _state: State<'_, AgentRunExecutorState>,
    params: RenderCommandParams,
) -> TauriApiResult<CommandRenderResult> {
    let Some(cfg) = CommandConfigLoader::get(&params.name) else {
        return Ok(api_error!("agent.command_not_found"));
    };

    Ok(api_success!(CommandConfigLoader::render(
        cfg,
        &params.input
    )))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSkillsParams {
    pub workspace_path: String,
}

/// List global + workspace skills (workspace has higher priority)
#[tauri::command]
pub async fn agent_list_skills(
    _state: State<'_, AgentRunExecutorState>,
    config_paths: State<'_, crate::config::paths::ConfigPaths>,
    params: ListSkillsParams,
) -> TauriApiResult<Vec<SkillSummary>> {
    let workspace_root = std::path::PathBuf::from(params.workspace_path);
    let global_skills_dir = config_paths.skills_dir();

    // Use new SkillManager API (global + workspace)
    let skill_manager = crate::agent::skill::SkillManager::new();
    let metadata_list = skill_manager
        .discover_skills(Some(global_skills_dir), Some(&workspace_root))
        .await
        .map_err(|err| {
            tracing::error!("❌ List skills failed: {err}");
            err
        });

    let metadata_list = match metadata_list {
        Ok(metadata_list) => metadata_list,
        Err(_) => return Ok(api_error!("agent.list_skills_failed")),
    };

    let mut out: Vec<SkillSummary> = metadata_list.iter().map(|m| m.into()).collect();

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(api_success!(out))
}

/// Validate Skill format
#[tauri::command]
pub async fn agent_validate_skill(
    _state: State<'_, AgentRunExecutorState>,
    skill_path: String,
) -> TauriApiResult<crate::agent::skill::ValidationResult> {
    let skill_dir = std::path::PathBuf::from(skill_path);

    match crate::agent::skill::SkillValidator::validate(&skill_dir).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("❌ Validate skill failed: {}", e);
            Ok(api_error!("agent.validate_skill_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSessionAgentParams {
    pub thread_id: i64,
    pub agent_type: String,
    pub reason: Option<String>,
}

/// Switch session's current Agent (no backward compatibility)
#[tauri::command]
pub async fn agent_switch_session_agent(
    state: State<'_, AgentRunExecutorState>,
    params: SwitchSessionAgentParams,
) -> TauriApiResult<EmptyData> {
    let persistence = state.executor.agent_persistence();
    let thread = match persistence.threads().get(params.thread_id).await {
        Ok(Some(thread)) => thread,
        Ok(None) => return Ok(api_error!("workspace.thread_not_found")),
        Err(err) => {
            tracing::error!("❌ Load thread failed during agent switch: {}", err);
            return Ok(api_error!("agent.switch_failed"));
        }
    };

    let workspace_root = std::path::PathBuf::from(thread.workspace_path.clone());
    let agent_configs = match AgentConfigLoader::load_for_workspace(&workspace_root).await {
        Ok(agent_configs) => agent_configs,
        Err(err) => {
            tracing::error!("❌ Load agent configs failed: {err}");
            return Ok(api_error!("agent.switch_failed"));
        }
    };
    if !agent_configs.contains_key(params.agent_type.as_str()) {
        return Ok(api_error!("agent.unknown_agent_type"));
    }

    let from_agent = thread.agent_type.clone();
    if from_agent == params.agent_type {
        return Ok(api_success!());
    }

    if let Err(err) = persistence
        .threads()
        .update_agent_type(params.thread_id, &params.agent_type)
        .await
    {
        tracing::error!("❌ Switch agent failed: {}", err);
        return Ok(api_error!("agent.switch_failed"));
    }

    let mut message = match persistence
        .messages()
        .create(CreateMessageParams {
            thread_id: params.thread_id,
            role: MessageRole::Assistant,
            status: MessageStatus::Completed,
            blocks: vec![Block::AgentSwitch(AgentSwitchBlock {
                from_agent,
                to_agent: params.agent_type.clone(),
                reason: params.reason.clone(),
            })],
            is_summary: false,
            is_internal: false,
            agent_type: &params.agent_type,
            parent_message_id: None,
            model_id: None,
            provider_id: None,
        })
        .await
    {
        Ok(m) => m,
        Err(err) => {
            tracing::error!("❌ Create switch message failed: {}", err);
            return Ok(api_error!("agent.switch_failed"));
        }
    };

    let now = chrono::Utc::now();
    message.finished_at = Some(now);
    message.duration_ms = Some(0);
    if let Err(err) = persistence.messages().update(&message).await {
        tracing::warn!("⚠️  Update switch message failed: {}", err);
    }

    Ok(api_success!())
}
