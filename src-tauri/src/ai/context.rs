/*!
 * AI会话上下文管理模块
 *
 * 负责构建AI请求的上下文、处理截断重问等功能
 */

use crate::ai::types::{AIConfig, AIContext, Message};
use crate::storage::sqlite::SqliteManager;
use crate::utils::error::AppResult;
use tracing::{debug, info, warn};

/// 构建AI请求的上下文
///
/// 根据会话ID和截断位置，动态获取历史消息列表。
/// 注意：当前版本不包含任何压缩逻辑，直接返回所有相关消息。
/// TODO: 未来在此处实现上下文智能压缩功能 (Phase 5)。
pub async fn build_context_for_request(
    sqlite_manager: &SqliteManager,
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    _config: &AIConfig, // config暂时未使用，但保留接口
) -> AppResult<Vec<Message>> {
    debug!(
        "构建上下文: conversation_id={}, up_to_message_id={:?}",
        conversation_id, up_to_message_id
    );

    // 直接获取历史消息并返回，不进行任何压缩
    let messages = if let Some(msg_id) = up_to_message_id {
        sqlite_manager
            .get_messages_up_to(conversation_id, msg_id)
            .await?
    } else {
        sqlite_manager
            .get_messages(conversation_id, None, None)
            .await?
    };

    info!(
        "构建上下文完成: conversation_id={}, 消息数量={}",
        conversation_id,
        messages.len()
    );

    Ok(messages)
}

/// 将历史消息转换为AIContext结构
///
/// 将数据库中的消息列表转换为AI请求所需的上下文结构
pub fn messages_to_ai_context(messages: Vec<Message>, _conversation_id: i64) -> AIContext {
    debug!("转换消息为AI上下文: 消息数量={}", messages.len());

    AIContext {
        chat_history: Some(messages),
        ..Default::default()
    }
}

/// 处理截断重新提问
///
/// 删除指定消息ID之后的所有消息，并更新会话统计
pub async fn handle_truncate_conversation(
    sqlite_manager: &SqliteManager,
    conversation_id: i64,
    truncate_after_message_id: i64,
) -> AppResult<()> {
    info!(
        "处理截断重问: conversation_id={}, truncate_after={}",
        conversation_id, truncate_after_message_id
    );

    // 1. 删除截断点之后的消息
    let deleted_count = sqlite_manager
        .delete_messages_after(conversation_id, truncate_after_message_id)
        .await?;

    // 2. 更新会话统计（触发器会自动处理message_count）
    if deleted_count > 0 {
        // 获取最后一条消息作为预览
        if let Some(last_message) = sqlite_manager.get_last_message(conversation_id).await? {
            let preview = truncate_string(&last_message.content, 40);
            sqlite_manager
                .update_conversation_preview(conversation_id, &preview)
                .await?;
        }
    }

    info!(
        "会话 {} 截断完成，删除了 {} 条消息",
        conversation_id, deleted_count
    );
    Ok(())
}

/// 截断字符串到指定长度
pub fn truncate_string(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}

/// 上下文压缩策略（Phase 5 预留功能）
#[allow(dead_code)]
pub enum CompressionStrategy {
    /// 保留首尾消息
    KeepFirstAndLast,
    /// 语义压缩（需要embedding模型）
    SemanticCompression,
    /// 基于重要性评分
    ImportanceScoring,
}

/// 上下文压缩配置（Phase 5 预留功能）
#[allow(dead_code)]
pub struct CompressionConfig {
    pub strategy: CompressionStrategy,
    pub max_tokens: u32,
    pub preserve_system_messages: bool,
    pub preserve_recent_messages: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            strategy: CompressionStrategy::KeepFirstAndLast,
            max_tokens: 4096,
            preserve_system_messages: true,
            preserve_recent_messages: 10,
        }
    }
}

/// 智能压缩上下文（Phase 5 预留功能）
///
/// TODO: 实现智能压缩逻辑
/// - 语义相似度计算
/// - 重要性评分
/// - 动态压缩策略选择
#[allow(dead_code)]
pub async fn compress_context(
    messages: Vec<Message>,
    _config: &CompressionConfig,
) -> AppResult<Vec<Message>> {
    warn!("智能压缩功能尚未实现，返回原始消息列表");

    // Phase 5: 在此处实现智能压缩逻辑
    // 1. 计算每条消息的重要性评分
    // 2. 根据策略选择保留的消息
    // 3. 生成压缩摘要

    Ok(messages)
}

/// 计算消息重要性评分（Phase 5 预留功能）
#[allow(dead_code)]
pub fn calculate_message_importance(_message: &Message) -> f64 {
    // TODO: 实现重要性评分算法
    // 考虑因素：
    // - 消息长度
    // - 关键词密度
    // - 用户交互频率
    // - 时间衰减因子

    1.0 // 默认评分
}

/// 生成会话摘要（Phase 5 预留功能）
#[allow(dead_code)]
pub async fn generate_conversation_summary(
    _messages: &[Message],
    _max_length: usize,
) -> AppResult<String> {
    // TODO: 实现会话摘要生成
    // 可能需要调用AI模型来生成摘要

    Ok("会话摘要功能尚未实现".to_string())
}
