/*!
 * AI会话上下文管理模块 - 增强版
 *
 * 集成智能上下文管理，支持压缩和优化
 */

use crate::ai::enhanced_context::{create_context_manager, ContextManager};
use crate::ai::types::{AIConfig, AIContext, Message};
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::AppResult;
use std::sync::LazyLock;
use tracing::debug;

// 全局上下文管理器实例
pub static CONTEXT_MANAGER: LazyLock<ContextManager> = LazyLock::new(|| create_context_manager());

/// 构建AI请求的智能上下文 - 新版本
///
/// 使用智能压缩和循环检测，替代简单的历史拼接
pub async fn build_context_for_request(
    repositories: &RepositoryManager,
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    _config: &AIConfig,
) -> AppResult<Vec<Message>> {
    debug!(
        "构建智能上下文: conversation_id={}, up_to_message_id={:?}",
        conversation_id, up_to_message_id
    );

    let context_result = CONTEXT_MANAGER
        .build_context(repositories, conversation_id, up_to_message_id)
        .await?;

    debug!(
        "智能上下文构建完成: 原始={}, 优化后={}, 压缩={}",
        context_result.original_count,
        context_result.messages.len(),
        context_result.compressed
    );

    Ok(context_result.messages)
}

pub async fn build_intelligent_prompt(
    repositories: &RepositoryManager,
    conversation_id: i64,
    current_message: &str,
    up_to_message_id: Option<i64>,
    current_working_directory: Option<&str>,
) -> AppResult<String> {
    CONTEXT_MANAGER
        .build_prompt(
            repositories,
            conversation_id,
            current_message,
            up_to_message_id,
            current_working_directory,
        )
        .await
}

pub async fn build_intelligent_prompt_with_tags(
    repositories: &RepositoryManager,
    conversation_id: i64,
    current_message: &str,
    up_to_message_id: Option<i64>,
    current_working_directory: Option<&str>,
    tag_context: Option<serde_json::Value>,
) -> AppResult<String> {
    CONTEXT_MANAGER
        .build_prompt_with_tags(
            repositories,
            conversation_id,
            current_message,
            up_to_message_id,
            current_working_directory,
            tag_context,
        )
        .await
}

pub fn messages_to_ai_context(messages: Vec<Message>, _conversation_id: i64) -> AIContext {
    debug!("转换消息为AI上下文: 消息数量={}", messages.len());

    AIContext {
        chat_history: Some(messages),
        ..Default::default()
    }
}

/// 删除指定消息ID之后的所有消息，并更新会话统计
pub async fn handle_truncate_conversation(
    repositories: &RepositoryManager,
    conversation_id: i64,
    truncate_after_message_id: i64,
) -> AppResult<()> {
    debug!(
        "处理截断重问: conversation_id={}, truncate_after={}",
        conversation_id, truncate_after_message_id
    );

    // 1. 删除截断点之后的消息
    let deleted_count = repositories
        .conversations()
        .delete_messages_after(conversation_id, truncate_after_message_id)
        .await?;

    // 2. 更新会话统计（触发器会自动处理message_count）
    // 删除消息后，会话统计会通过数据库触发器自动更新

    debug!(
        "会话 {} 截断完成，删除了 {} 条消息",
        conversation_id, deleted_count
    );
    Ok(())
}

pub fn truncate_string(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}
