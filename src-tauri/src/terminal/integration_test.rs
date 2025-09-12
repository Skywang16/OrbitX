/*!
 * 终端事件系统集成测试
 *
 * 测试统一事件处理器与终端上下文系统的集成
 */

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    use crate::mux::PaneId;
    use crate::terminal::{
        ActiveTerminalContextRegistry, TerminalContextEvent, TerminalContextService,
    };

    #[tokio::test]
    async fn test_event_integration_flow() {
        // 创建终端上下文注册表
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // 订阅事件
        let mut event_receiver = registry.subscribe_events();

        // 设置活跃面板
        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 验证事件被发送
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("应该接收到事件")
            .expect("事件接收不应该失败");

        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("应该接收到 ActivePaneChanged 事件"),
        }
    }

    #[tokio::test]
    async fn test_context_service_event_integration() {
        // 创建终端上下文服务
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let _context_service = TerminalContextService::default();

        // 订阅事件
        let mut event_receiver = registry.subscribe_events();

        // 设置活跃面板
        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 验证活跃面板变化事件
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("应该接收到事件")
            .expect("事件接收不应该失败");

        assert!(matches!(
            event,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));

        // 验证上下文服务可以获取活跃面板
        assert_eq!(registry.terminal_context_get_active_pane(), Some(pane_id));
    }

    #[tokio::test]
    async fn test_multiple_event_subscribers() {
        // 创建终端上下文注册表
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // 创建多个订阅者
        let mut receiver1 = registry.subscribe_events();
        let mut receiver2 = registry.subscribe_events();

        // 设置活跃面板
        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 验证所有订阅者都收到事件
        let event1 = timeout(Duration::from_millis(100), receiver1.recv())
            .await
            .expect("订阅者1应该接收到事件")
            .expect("事件接收不应该失败");

        let event2 = timeout(Duration::from_millis(100), receiver2.recv())
            .await
            .expect("订阅者2应该接收到事件")
            .expect("事件接收不应该失败");

        // 验证事件内容相同
        assert!(matches!(
            event1,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));
        assert!(matches!(
            event2,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));
    }

    #[tokio::test]
    async fn test_event_handler_conversion_functions() {
        use crate::mux::MuxNotification;
        use crate::terminal::event_handler::TerminalEventHandler;

        // 测试 Mux 通知转换
        let pane_id = PaneId::new(1);
        let notification = MuxNotification::PaneAdded(pane_id);

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);

        assert_eq!(event_name, "terminal_created");
        assert_eq!(payload["paneId"], 1);

        // 测试上下文事件转换
        let context_event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id: None,
            new_pane_id: Some(pane_id),
        };

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::context_event_to_tauri_event(&context_event);

        assert_eq!(event_name, "active_pane_changed");
        assert_eq!(payload["oldPaneId"], serde_json::Value::Null);
        assert_eq!(payload["newPaneId"], 1);
    }

    #[tokio::test]
    async fn test_event_deduplication() {
        // 创建终端上下文注册表
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // 订阅事件
        let mut event_receiver = registry.subscribe_events();

        let pane_id = PaneId::new(1);

        // 设置相同的活跃面板两次
        registry.terminal_context_set_active_pane(pane_id).unwrap();
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 应该只收到一个事件（第一次设置）
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("应该接收到第一个事件")
            .expect("事件接收不应该失败");

        assert!(matches!(
            event,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));

        // 第二个事件应该超时（因为没有变化）
        let result = timeout(Duration::from_millis(50), event_receiver.recv()).await;
        assert!(result.is_err(), "不应该收到重复的事件");
    }
}
