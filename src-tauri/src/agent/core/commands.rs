/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::core::executor::{ExecuteTaskParams, ExecuteTaskTreeParams, TaskExecutor, TaskSummary};
use crate::agent::events::TaskProgressPayload;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
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
    channel: Channel<TaskProgressPayload>,
) -> TauriApiResult<EmptyData> {
    match state.executor.execute_task(params, channel).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("执行Agent任务失败: {}", e);
            Ok(api_error!("agent.execute_failed"))
        }
    }
}

/// 执行Agent任务树（先Plan，按需Tree，再按父节点串行执行并传递总结）
#[tauri::command]
pub async fn agent_execute_task_tree(
    state: State<'_, TaskExecutorState>,
    params: ExecuteTaskTreeParams,
    channel: Channel<TaskProgressPayload>,
) -> TauriApiResult<EmptyData> {
    match state.executor.execute_task_tree(params, channel).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("执行Agent任务树失败: {}", e);
            Ok(api_error!("agent.execute_tree_failed"))
        }
    }
}

/// 暂停任务
#[tauri::command]
pub async fn agent_pause_task(
    state: State<'_, TaskExecutorState>,
    task_id: String,
) -> TauriApiResult<EmptyData> {
    match state.executor.pause_task(&task_id).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("暂停任务失败: {}", e);
            Ok(api_error!("agent.pause_failed"))
        }
    }
}

/// 恢复任务
#[tauri::command]
pub async fn agent_resume_task(
    state: State<'_, TaskExecutorState>,
    task_id: String,
    channel: Channel<TaskProgressPayload>,
) -> TauriApiResult<EmptyData> {
    match state.executor.resume_task(&task_id, channel).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("恢复任务失败: {}", e);
            Ok(api_error!("agent.resume_failed"))
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
            tracing::error!("取消任务失败: {}", e);
            Ok(api_error!("agent.cancel_failed"))
        }
    }
}

/// 列出任务
#[tauri::command]
pub async fn agent_list_tasks(
    state: State<'_, TaskExecutorState>,
    conversation_id: Option<i64>,
    status_filter: Option<String>,
) -> TauriApiResult<Vec<TaskSummary>> {
    match state
        .executor
        .list_tasks(conversation_id, status_filter)
        .await
    {
        Ok(tasks) => Ok(api_success!(tasks)),
        Err(e) => {
            tracing::error!("列出任务失败: {}", e);
            Ok(api_error!("agent.list_failed"))
        }
    }
}
