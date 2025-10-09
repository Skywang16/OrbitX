/*!
 * Shell Integration与Context Service集成测试
 */

#[cfg(test)]
mod tests {
    use crate::mux::{PaneId, TerminalMux};
    use crate::shell::{ShellIntegrationManager, ShellType};
    use crate::storage::cache::UnifiedCache;
    use crate::terminal::{
        context_registry::ActiveTerminalContextRegistry, context_service::TerminalContextService,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_complete_integration_flow() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = Arc::new(TerminalMux::new());

        let context_service = TerminalContextService::new_with_integration(
            registry.clone(),
            shell_integration.clone(),
            terminal_mux.clone(),
            Arc::new(UnifiedCache::new()),
        );

        let pane_id = PaneId::new(1);

        // 1. 设置活跃终端
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 2. 通过Shell集成管理器更新状态
        shell_integration.set_pane_shell_type(pane_id, ShellType::Bash);
        shell_integration.update_current_working_directory(pane_id, "/test/path".to_string());
        shell_integration.enable_integration(pane_id);

        // 3. 验证上下文服务的回退机制
        // 注意：由于面板在TerminalMux中不存在，这会回退到默认上下文
        let result = context_service
            .get_context_with_fallback(Some(pane_id))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string())); // 默认值
        assert!(matches!(
            context.shell_type,
            Some(crate::terminal::types::ShellType::Bash)
        ));

        // 4. 确保缓存失效调用不会panic
        context_service.invalidate_cache_entry(pane_id).await;
        shell_integration.update_current_working_directory(pane_id, "/new/path".to_string());
        sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_shell_integration_events() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = Arc::new(TerminalMux::new());

        let _context_service = TerminalContextService::new_with_integration(
            registry.clone(),
            shell_integration.clone(),
            terminal_mux.clone(),
            Arc::new(UnifiedCache::new()),
        );

        let pane_id = PaneId::new(1);

        // 订阅事件
        let mut event_receiver = registry.subscribe_events();

        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 接收活跃终端变化事件
        let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("应该收到事件")
            .expect("事件接收成功");

        match event {
            crate::terminal::types::TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("收到了错误的事件类型"),
        }

        // 通过Shell集成启用集成状态
        shell_integration.enable_integration(pane_id);

        // 但我们可以验证状态确实被更新了
        assert!(shell_integration.is_integration_enabled(pane_id));
    }

    #[test]
    fn test_performance_optimizations() {
        let manager = ShellIntegrationManager::new();
        let pane_ids: Vec<PaneId> = (1..=10).map(PaneId::new).collect();

        for &pane_id in &pane_ids {
            manager.set_pane_shell_type(pane_id, ShellType::Bash);
            manager.update_current_working_directory(pane_id, format!("/path/{}", pane_id));
        }

        // 测试批量获取性能
        let start = std::time::Instant::now();
        let states = manager.get_multiple_pane_states(&pane_ids);
        let duration = start.elapsed();

        // 验证结果
        assert_eq!(states.len(), pane_ids.len());
        for &pane_id in &pane_ids {
            assert!(states.contains_key(&pane_id));
            let state = &states[&pane_id];
            assert_eq!(state.shell_type, Some(ShellType::Bash));
            assert_eq!(
                state.current_working_directory,
                Some(format!("/path/{}", pane_id))
            );
        }

        // 性能应该很快（这是一个粗略的检查）
        assert!(duration < Duration::from_millis(10));

        // 测试活跃面板ID列表获取
        let active_panes = manager.get_active_pane_ids();
        assert_eq!(active_panes.len(), pane_ids.len());
        for &pane_id in &pane_ids {
            assert!(active_panes.contains(&pane_id));
        }
    }
}
