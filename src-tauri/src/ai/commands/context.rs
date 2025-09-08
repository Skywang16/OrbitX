/*!
 * AI上下文管理命令
 *
 * 负责智能上下文、终端上下文和缓存管理功能
 */

use super::AIManagerState;
use crate::ai::types::Message;
use crate::mux::PaneId;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success, validate_not_empty};
use tauri::State;

// ===== 智能上下文管理命令 =====

/// 获取压缩上下文（供前端eko使用）
#[tauri::command]
pub async fn get_compressed_context(
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<Vec<Message>> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let repositories = state.repositories();

    // 使用context.rs中的build_context_for_request函数
    let config = crate::ai::types::AIConfig::default();
    let _context_manager = state.get_context_manager();
    match crate::ai::context::build_context_for_request(
        repositories,
        conversation_id,
        up_to_message_id,
        &config,
    )
    .await
    {
        Ok(messages) => Ok(api_success!(messages)),
        Err(_) => Ok(api_error!("ai.get_context_failed")),
    }
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
) -> TauriApiResult<String> {
    // 参数验证
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }
    validate_not_empty!(current_message, "common.content_empty");

    let repositories = state.repositories();
    let context_service = state.get_terminal_context_service();

    // 使用 TerminalContextService 解析上下文
    let terminal_context = if let Some(pane_id) = pane_id {
        match context_service
            .get_context_by_pane(PaneId::new(pane_id))
            .await
        {
            Ok(context) => context,
            Err(_) => return Ok(api_error!("ai.get_context_failed")),
        }
    } else {
        match context_service.get_context_with_fallback(None).await {
            Ok(context) => context,
            Err(_) => return Ok(api_error!("ai.get_context_failed")),
        }
    };

    // 使用智能上下文管理器构建prompt，传递终端上下文
    let _context_manager = state.get_context_manager();
    match crate::ai::context::build_intelligent_prompt_with_context(
        repositories,
        conversation_id,
        &current_message,
        up_to_message_id,
        &terminal_context,
        tag_context,
    )
    .await
    {
        Ok(intelligent_prompt) => Ok(api_success!(intelligent_prompt)),
        Err(_) => Ok(api_error!("ai.build_prompt_failed")),
    }
}

// ===== 智能上下文配置管理命令 =====

/// 获取上下文配置
#[tauri::command]
pub async fn get_context_config() -> TauriApiResult<crate::ai::enhanced_context::ContextConfig> {
    Ok(api_success!(
        crate::ai::enhanced_context::ContextConfig::default()
    ))
}

/// 更新上下文配置
#[tauri::command]
pub async fn update_context_config(
    _config: crate::ai::enhanced_context::ContextConfig,
) -> TauriApiResult<EmptyData> {
    // TODO: 实现配置持久化
    // 可以保存到数据库或配置文件

    Ok(api_success!())
}

// ===== 缓存管理命令 =====

/// 获取KV缓存统计
#[tauri::command]
pub async fn get_kv_cache_stats(
    state: State<'_, AIManagerState>,
) -> TauriApiResult<crate::ai::enhanced_context::CacheStats> {
    let context_manager = state.get_context_manager();
    Ok(api_success!(context_manager.cache_stats()))
}

/// 清理过期的KV缓存
#[tauri::command]
pub async fn cleanup_kv_cache(state: State<'_, AIManagerState>) -> TauriApiResult<EmptyData> {
    let context_manager = state.get_context_manager();
    context_manager.cleanup_cache();
    Ok(api_success!())
}

/// 手动失效指定对话的缓存
#[tauri::command]
pub async fn invalidate_conversation_cache(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if conversation_id <= 0 {
        return Ok(api_error!("common.invalid_id"));
    }

    let context_manager = state.get_context_manager();
    context_manager.invalidate_cache(conversation_id);
    Ok(api_success!())
}
