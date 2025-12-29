/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::context::SummaryResult;
use crate::agent::core::executor::{
    ExecuteTaskParams, FileContextStatus, TaskExecutor, TaskSummary,
};
use crate::agent::events::TaskProgressPayload;
use crate::agent::ui::{UiConversation, UiMessage};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::{ipc::Channel, State};

/// TaskExecutor状态管理
pub struct TaskExecutorState {
    pub executor: Arc<TaskExecutor>,
}

#[tauri::command]
pub async fn agent_delete_conversation(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
) -> TauriApiResult<EmptyData> {
    let persistence = state.executor.agent_persistence();
    match persistence.conversations().delete(conversation_id).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to delete conversation: {}", e);
            Ok(api_error!("agent.conversation.delete_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_update_conversation_title(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
    title: String,
) -> TauriApiResult<EmptyData> {
    let persistence = state.executor.agent_persistence();
    match persistence
        .conversations()
        .update_title(conversation_id, &title)
        .await
    {
        Ok(_) => {
            if let Err(err) = state
                .executor
                .ui_persistence()
                .update_conversation_title(conversation_id, &title)
                .await
            {
                tracing::warn!("Failed to update UI conversation title: {}", err);
            }
            Ok(api_success!())
        }
        Err(e) => {
            tracing::error!("Failed to update conversation title: {}", e);
            Ok(api_error!("agent.conversation.update_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_ui_get_conversations(
    state: State<'_, TaskExecutorState>,
) -> TauriApiResult<Vec<UiConversation>> {
    match state.executor.ui_persistence().list_conversations().await {
        Ok(conversations) => Ok(api_success!(conversations)),
        Err(e) => {
            tracing::error!("Failed to list UI conversations: {}", e);
            Ok(api_error!("agent.ui.conversations_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_ui_get_messages(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
) -> TauriApiResult<Vec<UiMessage>> {
    match state
        .executor
        .ui_persistence()
        .get_messages(conversation_id)
        .await
    {
        Ok(messages) => Ok(api_success!(messages)),
        Err(e) => {
            tracing::error!("Failed to fetch UI messages: {}", e);
            Ok(api_error!("agent.ui.messages_failed"))
        }
    }
}

/// 删除指定消息及其之后的所有消息（用于回滚功能）
#[tauri::command]
pub async fn agent_ui_delete_messages_from(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
    message_id: i64,
) -> TauriApiResult<Option<String>> {
    match state
        .executor
        .ui_persistence()
        .delete_messages_from(conversation_id, message_id)
        .await
    {
        Ok(content) => Ok(api_success!(content)),
        Err(e) => {
            tracing::error!("Failed to delete messages from {}: {}", message_id, e);
            Ok(api_error!("agent.ui.delete_messages_failed"))
        }
    }
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
            tracing::error!("Failed to list tasks: {}", e);
            Ok(api_error!("agent.list_failed"))
        }
    }
}

/// 获取文件上下文状态
#[tauri::command]
pub async fn agent_get_file_context_status(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
) -> TauriApiResult<FileContextStatus> {
    match state
        .executor
        .get_file_context_status(conversation_id)
        .await
    {
        Ok(status) => Ok(api_success!(status)),
        Err(e) => {
            tracing::error!("Failed to fetch file context status: {}", e);
            Ok(api_error!("agent.context.file_status_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_get_user_rules(
    cache: State<'_, Arc<crate::storage::UnifiedCache>>,
) -> TauriApiResult<Option<String>> {
    let rules = cache.get_user_rules().await;
    Ok(api_success!(rules))
}

#[tauri::command]
pub async fn agent_set_user_rules(
    rules: Option<String>,
    cache: State<'_, Arc<crate::storage::UnifiedCache>>,
) -> TauriApiResult<EmptyData> {
    cache.set_user_rules(rules).await.ok();
    Ok(api_success!())
}

/// 手动触发会话摘要
#[tauri::command]
pub async fn agent_trigger_context_summary(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
    model_override: Option<String>,
) -> TauriApiResult<Option<SummaryResult>> {
    match state
        .executor
        .trigger_conversation_summary(conversation_id, model_override)
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("Failed to trigger conversation summary: {}", e);
            Ok(api_error!("agent.context.summary_failed"))
        }
    }
}

// === 双轨架构新增命令 ===

#[tauri::command]
pub async fn agent_create_conversation(
    state: State<'_, TaskExecutorState>,
    context_state: State<'_, crate::terminal::commands::TerminalContextState>,
    title: Option<String>,
) -> TauriApiResult<i64> {
    let title_clone = title.clone();

    // 自动从当前活跃终端获取工作目录
    let workspace_path = context_state
        .context_service
        .get_active_context()
        .await
        .ok()
        .and_then(|ctx| ctx.current_working_directory);

    match state
        .executor
        .create_conversation(title, workspace_path)
        .await
    {
        Ok(conversation_id) => {
            if let Err(err) = state
                .executor
                .ui_persistence()
                .ensure_conversation(conversation_id, title_clone.as_deref())
                .await
            {
                tracing::warn!(
                    "Failed to initialize UI conversation {}: {}",
                    conversation_id,
                    err
                );
            }
            Ok(api_success!(conversation_id))
        }
        Err(e) => {
            tracing::error!("Failed to create conversation: {}", e);
            Ok(api_error!("agent.conversation.create_failed"))
        }
    }
}
