/*!
 * TaskExecutor Tauri命令接口（已迁移至 agent/core/commands）
 */

use crate::agent::context::SummaryResult;
use crate::agent::core::executor::{
    ExecuteTaskParams, ExecuteTaskTreeParams, FileContextStatus, TaskExecutor, TaskSummary,
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
            tracing::error!("删除会话失败: {}", e);
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
                tracing::warn!("更新UI会话标题失败: {}", err);
            }
            Ok(api_success!())
        }
        Err(e) => {
            tracing::error!("更新会话标题失败: {}", e);
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
            tracing::error!("获取UI会话列表失败: {}", e);
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
            tracing::error!("获取UI消息失败: {}", e);
            Ok(api_error!("agent.ui.messages_failed"))
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

/// 获取文件上下文状态
#[tauri::command]
pub async fn agent_get_file_context_status(
    state: State<'_, TaskExecutorState>,
    conversation_id: i64,
) -> TauriApiResult<FileContextStatus> {
    match state
        .executor
        .fetch_file_context_status(conversation_id)
        .await
    {
        Ok(status) => Ok(api_success!(status)),
        Err(e) => {
            tracing::error!("获取文件上下文状态失败: {}", e);
            Ok(api_error!("agent.context.file_status_failed"))
        }
    }
}
#[tauri::command]
pub async fn agent_get_user_prefix_prompt(
    state: State<'_, TaskExecutorState>,
) -> TauriApiResult<Option<String>> {
    let repositories = state.executor.repositories();
    match repositories.ai_models().get_user_prefix_prompt().await {
        Ok(prompt) => Ok(api_success!(prompt)),
        Err(e) => {
            tracing::error!("获取前置提示词失败: {}", e);
            Ok(api_error!("agent.conversation.prefix_prompt_failed"))
        }
    }
}

#[tauri::command]
pub async fn agent_set_user_prefix_prompt(
    prompt: Option<String>,
    state: State<'_, TaskExecutorState>,
) -> TauriApiResult<EmptyData> {
    let repositories = state.executor.repositories();
    match repositories
        .ai_models()
        .set_user_prefix_prompt(prompt)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("设置前置提示词失败: {}", e);
            Ok(api_error!("agent.conversation.set_prefix_prompt_failed"))
        }
    }
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
            tracing::error!("触发会话摘要失败: {}", e);
            Ok(api_error!("agent.context.summary_failed"))
        }
    }
}

// === 双轨架构新增命令 ===

#[tauri::command]
pub async fn agent_create_conversation(
    state: State<'_, TaskExecutorState>,
    title: Option<String>,
    workspace_path: Option<String>,
) -> TauriApiResult<i64> {
    let title_clone = title.clone();
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
                tracing::warn!("初始化UI会话 {} 失败: {}", conversation_id, err);
            }
            Ok(api_success!(conversation_id))
        }
        Err(e) => {
            tracing::error!("创建会话失败: {}", e);
            Ok(api_error!("agent.conversation.create_failed"))
        }
    }
}
