/*!
 * Shell Integration and Context Service integration tests
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

    fn expected_fallback_cwd() -> String {
        dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "~".to_string())
    }

    #[tokio::test]
    async fn test_complete_integration_flow() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = TerminalMux::new_shared();

        let context_service = TerminalContextService::new_with_integration(
            registry.clone(),
            shell_integration.clone(),
            terminal_mux.clone(),
            Arc::new(UnifiedCache::new()),
        );

        let pane_id = PaneId::new(1);

        // 1. Set active terminal
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 2. Update state through Shell integration manager
        shell_integration.set_pane_shell_type(pane_id, ShellType::Bash);
        shell_integration.update_current_working_directory(pane_id, "/test/path".to_string());
        shell_integration.enable_integration(pane_id);

        // 3. Verify context service fallback mechanism
        // Note: Since pane doesn't exist in TerminalMux, this will fallback to default context
        let result = context_service
            .get_context_with_fallback(Some(pane_id))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(
            context.current_working_directory,
            Some(expected_fallback_cwd())
        );
        assert!(matches!(
            context.shell_type,
            Some(crate::terminal::types::ShellType::Bash)
        ));

        // 4. Ensure cache invalidation call doesn't panic
        context_service.invalidate_cache_entry(pane_id).await;
        shell_integration.update_current_working_directory(pane_id, "/new/path".to_string());
        sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_shell_integration_events() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = TerminalMux::new_shared();

        let _context_service = TerminalContextService::new_with_integration(
            registry.clone(),
            shell_integration.clone(),
            terminal_mux.clone(),
            Arc::new(UnifiedCache::new()),
        );

        let pane_id = PaneId::new(1);

        // Subscribe to events
        let mut event_receiver = registry.subscribe_events();

        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Receive active terminal change event
        let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("Should receive event")
            .expect("Event received successfully");

        match event {
            crate::terminal::TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("Received wrong event type"),
        }

        // Enable integration state through Shell integration
        shell_integration.enable_integration(pane_id);

        // But we can verify that the state was indeed updated
        assert!(shell_integration.is_integration_enabled(pane_id));
    }

    #[test]
    fn test_performance_optimizations() {
        let manager = ShellIntegrationManager::new();
        let pane_ids: Vec<PaneId> = (1..=10).map(PaneId::new).collect();

        for &pane_id in &pane_ids {
            manager.set_pane_shell_type(pane_id, ShellType::Bash);
            manager.update_current_working_directory(pane_id, format!("/path/{pane_id}"));
        }

        // Test batch retrieval performance
        let start = std::time::Instant::now();
        let states = manager.get_multiple_pane_states(&pane_ids);
        let duration = start.elapsed();

        // Verify results
        assert_eq!(states.len(), pane_ids.len());
        for &pane_id in &pane_ids {
            assert!(states.contains_key(&pane_id));
            let state = &states[&pane_id];
            assert_eq!(state.shell_type, Some(ShellType::Bash));
            assert_eq!(
                state.current_working_directory,
                Some(format!("/path/{pane_id}"))
            );
        }

        // Performance should be fast (this is a rough check)
        assert!(duration < Duration::from_millis(10));

        // Test active pane ID list retrieval
        let active_panes = manager.get_active_pane_ids();
        assert_eq!(active_panes.len(), pane_ids.len());
        for &pane_id in &pane_ids {
            assert!(active_panes.contains(&pane_id));
        }
    }
}
