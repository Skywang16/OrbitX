/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor, TaskSummary};
use crate::agent::tools::registry::ToolConfirmationDecision;
use crate::agent::types::TaskEvent;
use crate::agent::workspace_changes::{ChangeKind, PendingChange, WorkspaceChangeJournal};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{ipc::Channel, State};

/// TaskExecutor状态管理
pub struct TaskExecutorState {
    pub executor: Arc<TaskExecutor>,
}

impl TaskExecutorState {
    pub fn new(executor: Arc<TaskExecutor>) -> Self {
        Self { executor }
    }
}

/// 执行Agent任务
#[tauri::command]
pub async fn agent_execute_task(
    state: State<'_, TaskExecutorState>,
    changes: State<'_, Arc<WorkspaceChangeJournal>>,
    params: ExecuteTaskParams,
    channel: Channel<TaskEvent>,
) -> TauriApiResult<EmptyData> {
    let mut params = params;
    if let Ok(workspace_root) = std::path::PathBuf::from(&params.workspace_path).canonicalize() {
        let workspace_key: std::sync::Arc<str> =
            std::sync::Arc::from(workspace_root.to_string_lossy().to_string());
        let pending = changes.take_pending_by_key(workspace_key).await;
        let notice = build_file_change_notice(&pending);
        if !notice.is_empty() {
            params.user_prompt = format!("{}\n\n{}", notice, params.user_prompt);
        }
    }

    match state.executor.execute_task(params, channel).await {
        Ok(_context) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to execute Agent task: {}", e);
            Ok(api_error!("agent.execute_failed"))
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

    let mut lines = Vec::new();
    lines.push("WORKSPACE FILE CHANGES (since your last message)".to_string());
    lines.push(
        "These changes were made by the user or external tools. Treat them as authoritative."
            .to_string(),
    );
    lines.push("Do NOT revert them. If you need to edit any of these files, re-read first with read_file to avoid overwriting user changes.".to_string());
    lines.push(String::new());

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
            lines.push(format!("- {} ({}, {} ago): {}", path, kind, age, suffix));
            continue;
        }

        lines.push(format!("- {} ({}, {} ago):", path, kind, age));
        lines.push("```diff".to_string());
        lines.push(change.patch.clone().unwrap_or_default());
        lines.push("```".to_string());
    }

    let omitted = total_files.saturating_sub(shown);
    if omitted > 0 {
        lines.push(String::new());
        lines.push(format!(
            "(Plus {} more changed files omitted to keep context small.)",
            omitted
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
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{}h", hours);
    }
    let days = hours / 24;
    format!("{}d", days)
}

/// 取消任务
#[tauri::command]
pub async fn agent_cancel_task(
    state: State<'_, TaskExecutorState>,
    task_id: String,
    reason: Option<String>,
) -> TauriApiResult<EmptyData> {
    match state.executor.cancel_task(&task_id, reason).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to cancel task: {}", e);
            Ok(api_error!("agent.cancel_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmationParams {
    pub task_id: String,
    pub request_id: String,
    pub decision: ToolConfirmationDecision,
}

/// 回传工具确认结果
#[tauri::command]
pub async fn agent_tool_confirm(
    state: State<'_, TaskExecutorState>,
    params: ToolConfirmationParams,
) -> TauriApiResult<EmptyData> {
    let ctx = state
        .executor
        .active_tasks()
        .get(&params.task_id)
        .map(|entry| Arc::clone(entry.value()));

    let ctx = match ctx {
        Some(ctx) => ctx,
        None => return Ok(api_error!("agent.task_not_found")),
    };

    let ok = ctx
        .tool_registry()
        .resolve_confirmation(&params.request_id, params.decision);

    if ok {
        Ok(api_success!())
    } else {
        Ok(api_error!("agent.tool_confirm_not_found"))
    }
}

/// 列出任务
#[tauri::command]
pub async fn agent_list_tasks(
    state: State<'_, TaskExecutorState>,
    session_id: Option<i64>,
    status_filter: Option<String>,
) -> TauriApiResult<Vec<TaskSummary>> {
    match state.executor.list_tasks(session_id, status_filter).await {
        Ok(tasks) => Ok(api_success!(tasks)),
        Err(e) => {
            tracing::error!("Failed to list tasks: {}", e);
            Ok(api_error!("agent.list_failed"))
        }
    }
}
