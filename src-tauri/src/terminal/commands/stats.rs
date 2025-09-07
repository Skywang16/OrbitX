/*!
 * 终端统计信息管理命令
 *
 * 提供终端相关统计信息的获取功能，包括：
 * - 上下文缓存统计信息
 * - 活跃终端注册表统计信息
 * - 性能监控和调试支持
 */

use super::TerminalContextState;
use tauri::State;
use tracing::debug;

/// 获取上下文缓存统计信息
///
/// 返回当前上下文缓存的统计信息，包括缓存命中率、条目数量等。
/// 这对于监控和调试缓存性能很有用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(CacheStats)` - 缓存统计信息
/// * `Err(String)` - 获取失败的错误信息
#[tauri::command]
pub async fn get_context_cache_stats(
    state: State<'_, TerminalContextState>,
) -> Result<crate::terminal::CacheStats, String> {
    debug!("获取上下文缓存统计信息");

    let stats = state.context_service.get_cache_stats();

    debug!(
        "上下文缓存统计: 总条目={}, 命中率={:.2}%",
        stats.total_entries,
        stats.hit_rate * 100.0
    );

    Ok(stats)
}

/// 获取活跃终端注册表统计信息
///
/// 返回活跃终端注册表的统计信息，包括当前活跃终端、事件订阅者数量等。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(RegistryStats)` - 注册表统计信息
/// * `Err(String)` - 获取失败的错误信息
#[tauri::command]
pub async fn get_registry_stats(
    state: State<'_, TerminalContextState>,
) -> Result<crate::terminal::context_registry::RegistryStats, String> {
    debug!("获取活跃终端注册表统计信息");

    let stats = state.registry.get_stats();

    debug!(
        "注册表统计: 活跃终端={:?}, 订阅者数量={}",
        stats.global_active_pane, stats.event_subscriber_count
    );

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use crate::terminal::commands::tests::create_test_state;

    #[tokio::test]
    async fn test_get_registry_stats() {
        let state = create_test_state();

        // 测试获取注册表统计
        let stats = state.registry.get_stats();
        assert_eq!(stats.global_active_pane, None); // 初始状态没有活跃终端
        assert_eq!(stats.window_active_pane_count, 0);
    }
}
