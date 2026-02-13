/*!
 * 终端上下文缓存管理命令
 *
 * 提供终端上下文缓存的管理功能，包括：
 * - 使指定面板的缓存失效
 * - 清除所有缓存
 * - 缓存操作的日志记录
 */

use super::TerminalContextState;
use crate::mux::PaneId;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tauri::State;
use tracing::warn;

/// 使指定面板的上下文缓存失效
///
/// 强制清除指定面板的缓存上下文信息，下次查询时将重新获取最新数据。
/// 这在终端状态发生重大变化时很有用。
///
/// # Arguments
/// * `pane_id` - 要失效缓存的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 缓存失效成功
/// * `Err(String)` - 操作失败的错误信息
#[tauri::command]
pub async fn terminal_context_invalidate_cache(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<EmptyData> {
    if pane_id == 0 {
        warn!("面板ID不能为0");
        return Ok(api_error!("common.invalid_id"));
    }

    let pane_id = PaneId::new(pane_id);
    state.context_service.invalidate_cache_entry(pane_id).await;

    Ok(api_success!())
}

/// 清除所有上下文缓存
///
/// 清除所有终端的缓存上下文信息，强制下次查询时重新获取最新数据。
/// 这在系统重置或调试时很有用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 缓存清除成功
/// * `Err(String)` - 操作失败的错误信息
#[tauri::command]
pub async fn terminal_context_clear_all_cache(
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<EmptyData> {
    state.context_service.clear_all_cache().await;

    Ok(api_success!())
}

#[cfg(test)]
mod tests {
    use crate::mux::PaneId;
    use crate::terminal::commands::tests::create_test_state;

    #[tokio::test]
    async fn test_cache_operations() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 测试缓存失效操作
        state.context_service.invalidate_cache_entry(pane_id).await;

        // 测试清除所有缓存
        state.context_service.clear_all_cache().await;

        // 测试获取缓存统计
        let stats = state.context_service.get_cache_stats().await;
        assert_eq!(stats.total_entries, 0); // 初始状态应该没有缓存条目
    }
}
