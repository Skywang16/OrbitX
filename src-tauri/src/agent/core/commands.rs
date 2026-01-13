/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor, TaskSummary};
use crate::agent::tools::registry::ToolConfirmationDecision;
use crate::agent::types::TaskEvent;
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
    params: ExecuteTaskParams,
    channel: Channel<TaskEvent>,
) -> TauriApiResult<EmptyData> {
    match state.executor.execute_task(params, channel).await {
        Ok(_context) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to execute Agent task: {}", e);
            Ok(api_error!("agent.execute_failed"))
        }
    }
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
