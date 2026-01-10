/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::core::executor::{
    ExecuteTaskParams, FileContextStatus, TaskExecutor, TaskSummary,
};
use crate::agent::tools::registry::ToolConfirmationDecision;
use crate::agent::types::TaskEvent;
use crate::agent::compaction::{CompactionConfig, CompactionResult, CompactionService};
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};
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

/// 手动触发上下文压缩（Prune + 可选 Compact）
#[tauri::command]
pub async fn agent_trigger_compaction(
    state: State<'_, TaskExecutorState>,
    session_id: i64,
    model_override: Option<String>,
) -> TauriApiResult<CompactionResult> {
    use crate::agent::compaction::CompactionTrigger;
    use crate::storage::repositories::AIModels;

    let model_id = model_override.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

    let context_window = AIModels::new(&state.executor.database())
        .find_by_id(&model_id)
        .await
        .ok()
        .and_then(|m| m.and_then(|m| m.options))
        .and_then(|opts| opts.get("maxContextTokens").and_then(|v| v.as_u64()))
        .map(|v| v as u32)
        .unwrap_or(128_000);

    let service = CompactionService::new(
        state.executor.database(),
        state.executor.agent_persistence(),
        CompactionConfig::default(),
    );

    match service
        .prepare_compaction(session_id, context_window, CompactionTrigger::Manual)
        .await
    {
        Ok(prepared) => {
            if let Some(job) = prepared.summary_job {
                let service = service.clone();
                let model_id = model_id.clone();
                tokio::spawn(async move {
                    if let Err(err) = service.complete_summary_job(job, &model_id).await {
                        tracing::warn!("Manual compaction failed: {}", err);
                    }
                });
            }
            Ok(api_success!(prepared.result))
        }
        Err(e) => {
            tracing::error!("Failed to trigger compaction: {}", e);
            Ok(api_error!("agent.context.compaction_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_clear_compaction(
    state: State<'_, TaskExecutorState>,
    session_id: i64,
) -> TauriApiResult<EmptyData> {
    let service = CompactionService::new(
        state.executor.database(),
        state.executor.agent_persistence(),
        CompactionConfig::default(),
    );

    match service.clear_compaction_for_session(session_id).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to clear compaction: {}", e);
            Ok(api_error!("agent.context.compaction_clear_failed"))
        }
    }
}
