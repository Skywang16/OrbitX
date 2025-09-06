/*!
 * AI对话管理命令
 *
 * 负责对话的创建、管理和消息处理功能
 */

use super::AIManagerState;
use crate::ai::context::handle_truncate_conversation;
use crate::ai::types::{Conversation, Message};
use crate::storage::repositories::Repository;
use crate::utils::error::{ToTauriResult, Validator};

use tauri::State;
use tracing::debug;

// ===== AI会话管理命令 =====

/// 创建新会话
#[tauri::command]
pub async fn create_conversation(
    title: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<i64, String> {
    // 验证参数
    if let Some(ref t) = title {
        Validator::validate_not_empty(t, "会话标题")?;
    }

    let repositories = state.repositories();

    // 默认使用空标题，前端渲染时用 i18n 占位文案显示
    let conversation = Conversation::new(title.unwrap_or_else(|| "".to_string()));

    let conversation_id = repositories
        .conversations()
        .save(&conversation)
        .await
        .to_tauri()?;
    Ok(conversation_id)
}

/// 获取会话列表
#[tauri::command]
pub async fn get_conversations(
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<Conversation>, String> {
    debug!("获取会话列表: limit={:?}, offset={:?}", limit, offset);

    let repositories = state.repositories();

    let conversations = repositories
        .conversations()
        .find_conversations(limit, offset)
        .await
        .to_tauri()?;

    Ok(conversations)
}

/// 获取会话详情
#[tauri::command]
pub async fn get_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<Conversation, String> {
    debug!("获取会话详情: {}", conversation_id);

    let repositories = state.repositories();

    let conversation = repositories
        .conversations()
        .find_by_id(conversation_id)
        .await
        .to_tauri()?
        .ok_or_else(|| format!("会话不存在: {}", conversation_id))?;

    Ok(conversation)
}

/// 更新会话标题
#[tauri::command]
pub async fn update_conversation_title(
    conversation_id: i64,
    title: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    // 参数验证
    Validator::validate_id(conversation_id, "会话ID")?;
    Validator::validate_not_empty(&title, "会话标题")?;

    let repositories = state.repositories();

    repositories
        .conversations()
        .update_title(conversation_id, &title)
        .await
        .to_tauri()?;

    Ok(())
}

/// 删除会话
#[tauri::command]
pub async fn delete_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    // 参数验证
    Validator::validate_id(conversation_id, "会话ID")?;

    let repositories = state.repositories();

    repositories
        .conversations()
        .delete(conversation_id)
        .await
        .to_tauri()?;
    Ok(())
}

/// 截断会话（供前端eko使用）
#[tauri::command]
pub async fn truncate_conversation(
    conversation_id: i64,
    truncate_after_message_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if truncate_after_message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();

    // 截断会话
    handle_truncate_conversation(repositories, conversation_id, truncate_after_message_id)
        .await
        .to_tauri()?;

    Ok(())
}

// ===== 消息管理命令 =====

/// 保存单条消息（供前端eko使用）
#[tauri::command]
pub async fn save_message(
    conversation_id: i64,
    role: String,
    content: String,
    state: State<'_, AIManagerState>,
) -> Result<i64, String> {
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if content.trim().is_empty() {
        return Err("消息内容不能为空".to_string());
    }
    if !["user", "assistant", "system"].contains(&role.as_str()) {
        return Err("无效的消息角色".to_string());
    }

    let repositories = state.repositories();

    // 创建消息对象
    let message = Message::new(conversation_id, role, content);

    // 保存消息
    let message_id = repositories
        .conversations()
        .save_message(&message)
        .await
        .to_tauri()?;
    Ok(message_id)
}

/// 更新消息内容
#[tauri::command]
pub async fn update_message_content(
    message_id: i64,
    content: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_content(message_id, &content)
        .await
        .to_tauri()?;

    Ok(())
}

/// 更新消息步骤数据
#[tauri::command]
pub async fn update_message_steps(
    message_id: i64,
    steps_json: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_steps(message_id, &steps_json)
        .await
        .to_tauri()?;

    Ok(())
}

/// 更新消息状态
#[tauri::command]
pub async fn update_message_status(
    message_id: i64,
    status: Option<String>,
    duration_ms: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_status(message_id, status.as_deref(), duration_ms)
        .await
        .to_tauri()?;

    Ok(())
}
