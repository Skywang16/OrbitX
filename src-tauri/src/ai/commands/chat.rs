
use super::AIManagerState;
use crate::ai::context::handle_truncate_conversation;
use crate::ai::types::{Conversation, Message};
use crate::storage::repositories::Repository;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success, validate_not_empty};

use tauri::State;
use tracing::debug;

/// 创建新会话
#[tauri::command]
pub async fn ai_conversation_create(
    title: Option<String>,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<i64> {
    if let Some(ref t) = title {
        if t.trim().is_empty() {
            return Ok(api_error!("common.title_empty"));
        }
    }

    let repositories = state.repositories();

    let conversation = Conversation::new(title.unwrap_or_else(|| "".to_string()));

    match repositories.conversations().save(&conversation).await {
        Ok(conversation_id) => Ok(api_success!(conversation_id)),
        Err(_) => Ok(api_error!("ai.create_conversation_failed")),
    }
}

/// 获取会话列表
#[tauri::command]
pub async fn ai_conversation_get_all(
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<Vec<Conversation>> {
    debug!("获取会话列表: limit={:?}, offset={:?}", limit, offset);

    let repositories = state.repositories();

    match repositories
        .conversations()
        .find_conversations(limit, offset)
        .await
    {
        Ok(conversations) => Ok(api_success!(conversations)),
        Err(_) => Ok(api_error!("ai.get_conversations_failed")),
    }
}

/// 获取会话详情
#[tauri::command]
pub async fn ai_conversation_get(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<Conversation> {
    debug!("获取会话详情: {}", conversation_id);

    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();

    match repositories
        .conversations()
        .find_by_id(conversation_id)
        .await
    {
        Ok(Some(conversation)) => Ok(api_success!(conversation)),
        Ok(None) => Ok(api_error!("ai.conversation_not_found")),
        Err(_) => Ok(api_error!("ai.get_conversation_failed")),
    }
}

/// 更新会话标题
#[tauri::command]
pub async fn ai_conversation_update_title(
    conversation_id: i64,
    title: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }
    validate_not_empty!(title, "common.title_empty");

    let repositories = state.repositories();

    match repositories
        .conversations()
        .update_title(conversation_id, &title)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.update_title_failed")),
    }
}

/// 删除会话
#[tauri::command]
pub async fn ai_conversation_delete(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();

    match repositories.conversations().delete(conversation_id).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.delete_conversation_failed")),
    }
}

/// 截断会话
#[tauri::command]
pub async fn ai_conversation_truncate(
    conversation_id: i64,
    truncate_after_message_id: i64,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }
    if truncate_after_message_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();

    match handle_truncate_conversation(repositories, conversation_id, truncate_after_message_id)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.truncate_conversation_failed")),
    }
}

/// 保存单条消息
#[tauri::command]
pub async fn ai_conversation_save_message(
    conversation_id: i64,
    role: String,
    content: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<i64> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }
    validate_not_empty!(content, "common.content_empty");
    if !["user", "assistant", "system"].contains(&role.as_str()) {
        return Ok(api_error!("common.invalid_role"));
    }

    let repositories = state.repositories();

    let message = Message::new(conversation_id, role, content);

    match repositories.conversations().ai_conversation_save_message(&message).await {
        Ok(message_id) => Ok(api_success!(message_id)),
        Err(_) => Ok(api_error!("ai.save_message_failed")),
    }
}

/// 更新消息内容
#[tauri::command]
pub async fn ai_conversation_update_message_content(
    message_id: i64,
    content: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if message_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();
    match repositories
        .conversations()
        .ai_conversation_update_message_content(message_id, &content)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.update_message_failed")),
    }
}

/// 更新消息步骤数据
#[tauri::command]
pub async fn ai_conversation_update_message_steps(
    message_id: i64,
    steps_json: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if message_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();
    match repositories
        .conversations()
        .ai_conversation_update_message_steps(message_id, &steps_json)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.update_message_failed")),
    }
}

/// 更新消息状态
#[tauri::command]
pub async fn ai_conversation_update_message_status(
    message_id: i64,
    status: Option<String>,
    duration_ms: Option<i64>,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if message_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();
    match repositories
        .conversations()
        .ai_conversation_update_message_status(message_id, status.as_deref(), duration_ms)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.update_message_failed")),
    }
}
