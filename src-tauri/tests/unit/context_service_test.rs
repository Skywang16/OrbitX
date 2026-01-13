/*!
 * TerminalContextService 单元测试
 *
 * 测试缓存逻辑、TTL适配和缓存失效机制
 */

use std::sync::Arc;
use terminal_lib::mux::{PaneId, PtySize, TerminalMux};
use terminal_lib::shell::ShellIntegrationManager;
use terminal_lib::storage::cache::UnifiedCache;
use terminal_lib::terminal::{ActiveTerminalContextRegistry, TerminalContextService};

/// 创建测试用的context service
fn create_test_context_service() -> Arc<TerminalContextService> {
    let registry = Arc::new(ActiveTerminalContextRegistry::new());
    let shell_integration = Arc::new(ShellIntegrationManager::new());
    let terminal_mux = TerminalMux::new_shared_with_shell_integration(shell_integration.clone());
    let cache = Arc::new(UnifiedCache::new());

    TerminalContextService::new_with_integration(registry, shell_integration, terminal_mux, cache)
}

#[tokio::test]
async fn test_cache_stats_initialization() {
    let service = create_test_context_service();

    let stats = service.get_cache_stats().await;
    assert_eq!(stats.hit_count, 0);
    assert_eq!(stats.miss_count, 0);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[tokio::test]
async fn test_cache_miss_and_hit() {
    let service = create_test_context_service();
    let pane_id = PaneId::new(1);

    // 第一次查询应该miss
    let result = service.get_context_by_pane(pane_id).await;
    assert!(result.is_err(), "面板不存在应该返回错误");

    let stats = service.get_cache_stats().await;
    assert_eq!(stats.miss_count, 1);
    assert_eq!(stats.hit_count, 0);
}

#[tokio::test]
async fn test_cache_invalidation() {
    let service = create_test_context_service();
    let pane_id = PaneId::new(123);

    // 使失效不存在的缓存不应该panic
    service.invalidate_cache_entry(pane_id).await;

    let stats = service.get_cache_stats().await;
    assert_eq!(stats.total_entries, 0);
}

#[tokio::test]
async fn test_clear_all_cache() {
    let service = create_test_context_service();

    // 清空不存在的缓存不应该panic
    service.clear_all_cache().await;

    let stats = service.get_cache_stats().await;
    assert_eq!(stats.total_entries, 0);
}

#[tokio::test]
async fn test_get_active_context_no_active_pane() {
    let service = create_test_context_service();

    let result = service.get_active_context().await;
    assert!(result.is_err(), "没有活跃面板应该返回错误");
}

#[tokio::test]
async fn test_get_context_with_fallback() {
    let service = create_test_context_service();

    // 没有活跃面板也没有缓存时，应该返回默认上下文
    let result = service.get_context_with_fallback(None).await;
    assert!(result.is_ok(), "回退逻辑应该返回默认上下文");

    let context = result.unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));
}

#[tokio::test]
async fn test_cache_eviction_count() {
    let service = create_test_context_service();
    let pane_id = PaneId::new(456);

    // 失效缓存条目
    service.invalidate_cache_entry(pane_id).await;

    let stats = service.get_cache_stats().await;
    // 由于缓存条目不存在，不会增加eviction_count
    assert_eq!(stats.eviction_count, 0);
}

#[tokio::test]
async fn test_get_active_cwd_error() {
    let service = create_test_context_service();

    let result = service.get_active_cwd().await;
    assert!(result.is_err(), "没有活跃面板应该返回错误");
}

#[tokio::test]
async fn test_get_active_shell_type_error() {
    let service = create_test_context_service();

    let result = service.get_active_shell_type().await;
    assert!(result.is_err(), "没有活跃面板应该返回错误");
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    let service = Arc::new(create_test_context_service());
    let mut handles = Vec::new();

    // 并发访问缓存统计
    for _ in 0..10 {
        let service_clone = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let _stats = service_clone.get_cache_stats().await;
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }

    let stats = service.get_cache_stats().await;
    assert_eq!(stats.total_entries, 0);
}

#[tokio::test]
async fn test_context_service_integration() {
    let registry = Arc::new(ActiveTerminalContextRegistry::new());
    let shell_integration = Arc::new(ShellIntegrationManager::new());
    let terminal_mux = TerminalMux::new_shared_with_shell_integration(shell_integration.clone());
    let cache = Arc::new(UnifiedCache::new());

    let service = TerminalContextService::new_with_integration(
        registry.clone(),
        shell_integration.clone(),
        terminal_mux.clone(),
        cache.clone(),
    );

    // 创建一个实际的面板
    let pane_id = terminal_mux.create_pane(PtySize::default()).await.unwrap();

    // 设置为活跃面板
    registry.terminal_context_set_active_pane(pane_id).unwrap();

    // 现在应该能获取上下文
    let result = service.get_active_context().await;
    assert!(result.is_ok(), "应该能获取活跃面板的上下文");

    let context = result.unwrap();
    assert_eq!(context.pane_id, pane_id);

    // 清理
    terminal_mux.remove_pane(pane_id).unwrap();
}
