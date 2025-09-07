/*!
 * AI上下文管理命令
 *
 * 负责智能上下文、终端上下文和缓存管理功能
 */

use super::AIManagerState;
use crate::ai::types::Message;
use crate::mux::PaneId;
use crate::utils::error::ToTauriResult;
use anyhow::Context;
use tauri::State;

// ===== 智能上下文管理命令 =====

/// 获取压缩上下文（供前端eko使用）
#[tauri::command]
pub async fn get_compressed_context(
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<Message>, String> {
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    let repositories = state.repositories();

    // 使用context.rs中的build_context_for_request函数
    let config = crate::ai::types::AIConfig::default();
    let _context_manager = state.get_context_manager();
    let messages = crate::ai::context::build_context_for_request(
        repositories,
        conversation_id,
        up_to_message_id,
        &config,
    )
    .await
    .to_tauri()?;

    Ok(messages)
}

/// 构建带智能上下文的prompt（专门用于AI推理）
#[tauri::command]
pub async fn build_prompt_with_context(
    conversation_id: i64,
    current_message: String,
    up_to_message_id: Option<i64>,
    pane_id: Option<u32>,
    tag_context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<String, String> {
    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if current_message.trim().is_empty() {
        return Err("当前消息不能为空".to_string());
    }

    let repositories = state.repositories();
    let context_service = state.get_terminal_context_service();

    // 使用 TerminalContextService 解析上下文
    let terminal_context = if let Some(pane_id) = pane_id {
        context_service
            .get_context_by_pane(PaneId::new(pane_id))
            .await
            .context("获取终端上下文失败")
            .to_tauri()?
    } else {
        context_service
            .get_context_with_fallback(None)
            .await
            .context("获取活跃终端上下文失败")
            .to_tauri()?
    };

    // 使用智能上下文管理器构建prompt，传递终端上下文
    let _context_manager = state.get_context_manager();
    let intelligent_prompt = crate::ai::context::build_intelligent_prompt_with_context(
        repositories,
        conversation_id,
        &current_message,
        up_to_message_id,
        &terminal_context,
        tag_context,
    )
    .await
    .to_tauri()?;

    Ok(intelligent_prompt)
}

// ===== 智能上下文配置管理命令 =====

/// 获取上下文配置
#[tauri::command]
pub async fn get_context_config() -> Result<crate::ai::enhanced_context::ContextConfig, String> {
    Ok(crate::ai::enhanced_context::ContextConfig::default())
}

/// 更新上下文配置
#[tauri::command]
pub async fn update_context_config(
    _config: crate::ai::enhanced_context::ContextConfig,
) -> Result<(), String> {
    // TODO: 实现配置持久化
    // 可以保存到数据库或配置文件

    Ok(())
}

// ===== 缓存管理命令 =====

/// 获取KV缓存统计
#[tauri::command]
pub async fn get_kv_cache_stats(
    state: State<'_, AIManagerState>,
) -> Result<crate::ai::enhanced_context::CacheStats, String> {
    let context_manager = state.get_context_manager();
    Ok(context_manager.cache_stats())
}

/// 清理过期的KV缓存
#[tauri::command]
pub async fn cleanup_kv_cache(state: State<'_, AIManagerState>) -> Result<(), String> {
    let context_manager = state.get_context_manager();
    context_manager.cleanup_cache();
    Ok(())
}

/// 手动失效指定对话的缓存
#[tauri::command]
pub async fn invalidate_conversation_cache(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    let context_manager = state.get_context_manager();
    context_manager.invalidate_cache(conversation_id);
    Ok(())
}
