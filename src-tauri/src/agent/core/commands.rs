/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::context::SummaryResult;
use crate::agent::core::executor::{
    ExecuteTaskParams, FileContextStatus, TaskExecutor, TaskSummary,
};
use crate::agent::events::TaskProgressPayload;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};
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
        Ok(_context) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to execute Agent task: {}", e);
            Ok(api_error!("agent.execute_failed"))
        }
    }
}

/// 暂停任务
#[tauri::command]
pub async fn agent_pause_task(
    state: State<'_, TaskExecutorState>,
    task_id: String,
) -> TauriApiResult<EmptyData> {
    match state.executor.pause_task(&task_id, true).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to pause task: {}", e);
            Ok(api_error!("agent.pause_failed"))
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

/// 获取文件上下文状态
#[tauri::command]
pub async fn agent_get_file_context_status(
    state: State<'_, TaskExecutorState>,
    session_id: i64,
) -> TauriApiResult<FileContextStatus> {
    match state.executor.get_file_context_status(session_id).await {
        Ok(status) => Ok(api_success!(status)),
        Err(e) => {
            tracing::error!("Failed to fetch file context status: {}", e);
            Ok(api_error!("agent.context.file_status_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_get_user_rules(
    database: State<'_, Arc<DatabaseManager>>,
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<Option<String>> {
    match AppPreferences::new(&database).get("agent.user_rules").await {
        Ok(value) => {
            let _ = cache.set_user_rules(value.clone()).await;
            Ok(api_success!(value))
        }
        Err(e) => {
            tracing::error!("Failed to load user rules: {}", e);
            Ok(api_error!("agent.rules.load_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_set_user_rules(
    rules: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<EmptyData> {
    match AppPreferences::new(&database)
        .set("agent.user_rules", rules.as_deref())
        .await
    {
        Ok(_) => {
            let _ = cache.set_user_rules(rules).await;
            Ok(api_success!())
        }
        Err(e) => {
            tracing::error!("Failed to save user rules: {}", e);
            Ok(api_error!("agent.rules.save_failed"))
        }
    }
}

/// 手动触发会话摘要
#[tauri::command]
pub async fn agent_trigger_session_summary(
    state: State<'_, TaskExecutorState>,
    session_id: i64,
    model_override: Option<String>,
) -> TauriApiResult<Option<SummaryResult>> {
    match state
        .executor
        .trigger_session_summary(session_id, model_override)
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("Failed to trigger session summary: {}", e);
            Ok(api_error!("agent.context.summary_failed"))
        }
    }
}
