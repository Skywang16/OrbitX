//! TerminalMux 单元测试
//!
//! 测试面板创建、查找、移除功能以及通知系统

#[cfg(test)]
mod terminal_mux_tests {
    use terminal_lib::mux::{PaneId, PtySize, TerminalMux};
    use terminal_lib::terminal::event_handler::TerminalEventHandler;

    #[tokio::test]
    async fn test_create_single_pane() {
        let mux = TerminalMux::new();
        let size = PtySize::new(24, 80);

        let pane_id = mux.create_pane(size).await.unwrap();
        assert_eq!(pane_id.as_u32(), 1);
        assert_eq!(mux.pane_count(), 1);

        let pane = mux.get_pane(pane_id).unwrap();
        assert_eq!(pane.pane_id(), pane_id);
        assert_eq!(pane.get_size().rows, 24);
        assert_eq!(pane.get_size().cols, 80);
        assert!(!pane.is_dead());
    }

    #[tokio::test]
    async fn test_create_multiple_panes() {
        let mux = TerminalMux::new();

        let pane1 = mux.create_pane(PtySize::new(24, 80)).await.unwrap();
        let pane2 = mux.create_pane(PtySize::new(30, 100)).await.unwrap();
        let pane3 = mux.create_pane(PtySize::new(40, 120)).await.unwrap();

        // 验证面板ID是递增的
        assert_eq!(pane1.as_u32(), 1);
        assert_eq!(pane2.as_u32(), 2);
        assert_eq!(pane3.as_u32(), 3);

        // 验证面板数量
        assert_eq!(mux.pane_count(), 3);

        // 验证所有面板都存在
        assert!(mux.get_pane(pane1).is_some());
        assert!(mux.get_pane(pane2).is_some());
        assert!(mux.get_pane(pane3).is_some());

        // 验证面板列表
        let panes = mux.list_panes();
        assert_eq!(panes.len(), 3);
        assert!(panes.contains(&pane1));
        assert!(panes.contains(&pane2));
        assert!(panes.contains(&pane3));
    }

    #[tokio::test]
    async fn test_remove_pane() {
        let mux = TerminalMux::new();
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 验证面板存在
        assert!(mux.get_pane(pane_id).is_some());
        assert_eq!(mux.pane_count(), 1);

        // 移除面板
        mux.remove_pane(pane_id).unwrap();

        // 验证面板已移除
        assert!(mux.get_pane(pane_id).is_none());
        assert_eq!(mux.pane_count(), 0);

        // 再次移除应该失败
        assert!(mux.remove_pane(pane_id).is_err());
    }

    #[tokio::test]
    async fn test_remove_multiple_panes() {
        let mux = TerminalMux::new();

        let pane1 = mux.create_pane(PtySize::default()).await.unwrap();
        let pane2 = mux.create_pane(PtySize::default()).await.unwrap();
        let pane3 = mux.create_pane(PtySize::default()).await.unwrap();

        assert_eq!(mux.pane_count(), 3);

        // 移除中间的面板
        mux.remove_pane(pane2).unwrap();
        assert_eq!(mux.pane_count(), 2);
        assert!(mux.get_pane(pane1).is_some());
        assert!(mux.get_pane(pane2).is_none());
        assert!(mux.get_pane(pane3).is_some());

        // 移除剩余面板
        mux.remove_pane(pane1).unwrap();
        mux.remove_pane(pane3).unwrap();
        assert_eq!(mux.pane_count(), 0);
    }

    #[tokio::test]
    async fn test_pane_not_found_error() {
        let mux = TerminalMux::new();
        let nonexistent_pane = PaneId::new(999);

        // 获取不存在的面板
        assert!(mux.get_pane(nonexistent_pane).is_none());

        // 写入不存在的面板
        let result = mux.write_to_pane(nonexistent_pane, b"test");
        assert!(result.is_err());

        // 调整不存在面板的大小
        let result = mux.resize_pane(nonexistent_pane, PtySize::default());
        assert!(result.is_err());

        // 移除不存在的面板
        let result = mux.remove_pane(nonexistent_pane);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_to_pane() {
        let mux = TerminalMux::new();
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 写入字节数据
        let result = mux.write_to_pane(pane_id, b"echo hello\n");
        assert!(result.is_ok());

        // 写入更多数据
        let result = mux.write_to_pane(pane_id, b"ls -la\n");
        assert!(result.is_ok());

        // 写入空数据
        let result = mux.write_to_pane(pane_id, b"");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resize_pane() {
        let mux = TerminalMux::new();
        let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();

        // 验证初始大小
        let pane = mux.get_pane(pane_id).unwrap();
        let initial_size = pane.get_size();
        assert_eq!(initial_size.rows, 24);
        assert_eq!(initial_size.cols, 80);

        // 调整大小
        let new_size = PtySize::new(30, 120);
        let result = mux.resize_pane(pane_id, new_size);
        assert!(result.is_ok());

        // 验证大小已更新
        let pane = mux.get_pane(pane_id).unwrap();
        let current_size = pane.get_size();
        assert_eq!(current_size.rows, 30);
        assert_eq!(current_size.cols, 120);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let mux = TerminalMux::new();

        // 创建多个面板
        let _pane1 = mux.create_pane(PtySize::default()).await.unwrap();
        let _pane2 = mux.create_pane(PtySize::default()).await.unwrap();
        let _pane3 = mux.create_pane(PtySize::default()).await.unwrap();

        assert_eq!(mux.pane_count(), 3);

        // 关闭应该清理所有面板
        mux.shutdown().unwrap();
        assert_eq!(mux.pane_count(), 0);
    }
}

#[cfg(test)]
mod notification_system_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use terminal_lib::mux::{MuxNotification, PaneId, TerminalMux};

    #[test]
    fn test_subscribe_and_notify() {
        let mux = TerminalMux::new();
        let received_count = Arc::new(AtomicUsize::new(0));
        let received_clone = Arc::clone(&received_count);

        // 订阅通知
        let subscriber_id = mux.subscribe(move |notification| {
            if let MuxNotification::PaneAdded(_) = notification {
                received_clone.fetch_add(1, Ordering::Relaxed);
            }
            true // 继续订阅
        });

        // 发送通知
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        mux.notify(MuxNotification::PaneAdded(PaneId::new(2)));
        mux.notify(MuxNotification::PaneRemoved(PaneId::new(1))); // 不应该被计数

        // 给通知处理一些时间
        thread::sleep(Duration::from_millis(10));

        assert_eq!(received_count.load(Ordering::Relaxed), 2);

        // 取消订阅
        assert!(mux.unsubscribe(subscriber_id));
        assert!(!mux.unsubscribe(subscriber_id)); // 重复取消应该返回false
    }

    #[test]
    fn test_multiple_subscribers() {
        let mux = TerminalMux::new();
        let received1 = Arc::new(AtomicUsize::new(0));
        let received2 = Arc::new(AtomicUsize::new(0));
        let received3 = Arc::new(AtomicUsize::new(0));

        let received1_clone = Arc::clone(&received1);
        let received2_clone = Arc::clone(&received2);
        let received3_clone = Arc::clone(&received3);

        // 订阅者1：只关心PaneAdded
        let sub1 = mux.subscribe(move |notification| {
            if matches!(notification, MuxNotification::PaneAdded(_)) {
                received1_clone.fetch_add(1, Ordering::Relaxed);
            }
            true
        });

        // 订阅者2：只关心PaneRemoved
        let sub2 = mux.subscribe(move |notification| {
            if matches!(notification, MuxNotification::PaneRemoved(_)) {
                received2_clone.fetch_add(1, Ordering::Relaxed);
            }
            true
        });

        // 订阅者3：关心所有通知
        let sub3 = mux.subscribe(move |notification| {
            match notification {
                MuxNotification::PaneAdded(_)
                | MuxNotification::PaneRemoved(_)
                | MuxNotification::PaneOutput { .. } => {
                    received3_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            true
        });

        // 发送各种通知
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        mux.notify(MuxNotification::PaneAdded(PaneId::new(2)));
        mux.notify(MuxNotification::PaneRemoved(PaneId::new(1)));
        mux.notify(MuxNotification::PaneOutput {
            pane_id: PaneId::new(2),
            data: b"test".to_vec().into(),
        });

        thread::sleep(Duration::from_millis(10));

        assert_eq!(received1.load(Ordering::Relaxed), 2); // 2个PaneAdded
        assert_eq!(received2.load(Ordering::Relaxed), 1); // 1个PaneRemoved
        assert_eq!(received3.load(Ordering::Relaxed), 4); // 所有4个通知

        // 清理订阅者
        mux.unsubscribe(sub1);
        mux.unsubscribe(sub2);
        mux.unsubscribe(sub3);
    }

    #[test]
    fn test_subscriber_cleanup() {
        let mux = TerminalMux::new();
        let cleanup_count = Arc::new(AtomicUsize::new(0));
        let cleanup_clone = Arc::clone(&cleanup_count);

        // 添加一个返回false的订阅者（应该被清理）
        mux.subscribe(move |_| {
            cleanup_clone.fetch_add(1, Ordering::Relaxed);
            false // 请求取消订阅
        });

        // 添加一个正常的订阅者
        let normal_count = Arc::new(AtomicUsize::new(0));
        let normal_clone = Arc::clone(&normal_count);
        mux.subscribe(move |_| {
            normal_clone.fetch_add(1, Ordering::Relaxed);
            true // 继续订阅
        });

        // 发送通知
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        thread::sleep(Duration::from_millis(10));

        // 再次发送通知，第一个订阅者应该已经被清理
        mux.notify(MuxNotification::PaneAdded(PaneId::new(2)));
        thread::sleep(Duration::from_millis(10));

        // 第一个订阅者应该只收到一次通知
        assert_eq!(cleanup_count.load(Ordering::Relaxed), 1);
        // 第二个订阅者应该收到两次通知
        assert_eq!(normal_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_cross_thread_notification() {
        let mux = Arc::new(TerminalMux::new());
        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = Arc::clone(&received);

        // 订阅通知
        let _subscriber_id = mux.subscribe(move |notification| {
            if let MuxNotification::PaneOutput { .. } = notification {
                received_clone.fetch_add(1, Ordering::Relaxed);
            }
            true
        });

        // 从另一个线程发送通知
        let mux_clone = Arc::clone(&mux);
        let handle = thread::spawn(move || {
            for i in 0..5 {
                mux_clone.notify_from_any_thread(MuxNotification::PaneOutput {
                    pane_id: PaneId::new(1),
                    data: format!("test data {i}").into_bytes().into(),
                });
                thread::sleep(Duration::from_millis(1));
            }
        });

        handle.join().unwrap();

        // 处理跨线程通知
        for _ in 0..10 {
            mux.process_notifications();
            thread::sleep(Duration::from_millis(5));
        }

        // 验证通知被接收
        assert_eq!(received.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_notification_to_tauri_event() {
        // 测试PaneOutput通知转换
        let notification = MuxNotification::PaneOutput {
            pane_id: PaneId::new(1),
            data: b"hello world".to_vec().into(),
        };

        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_output");
        assert_eq!(payload["paneId"], 1);
        assert_eq!(payload["data"], "hello world");

        // 测试PaneAdded通知转换
        let notification = MuxNotification::PaneAdded(PaneId::new(2));
        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_created");
        assert_eq!(payload["paneId"], 2);

        // 测试PaneRemoved通知转换
        let notification = MuxNotification::PaneRemoved(PaneId::new(3));
        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_closed");
        assert_eq!(payload["paneId"], 3);

        // 测试PaneResized通知转换
        let notification = MuxNotification::PaneResized {
            pane_id: PaneId::new(4),
            size: terminal_lib::mux::PtySize::new(30, 100),
        };
        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_resized");
        assert_eq!(payload["paneId"], 4);
        assert_eq!(payload["rows"], 30);
        assert_eq!(payload["cols"], 100);

        // 测试PaneExited通知转换
        let notification = MuxNotification::PaneExited {
            pane_id: PaneId::new(5),
            exit_code: Some(0),
        };
        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_exit");
        assert_eq!(payload["paneId"], 5);
        assert_eq!(payload["exitCode"], 0);

        // 测试没有退出码的情况
        let notification = MuxNotification::PaneExited {
            pane_id: PaneId::new(6),
            exit_code: None,
        };
        let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
        assert_eq!(event_name, "terminal_exit");
        assert_eq!(payload["paneId"], 6);
        assert_eq!(payload["exitCode"], serde_json::Value::Null);
    }

    #[test]
    fn test_debug_subscriber() {
        let mux = TerminalMux::new();
        let subscriber = TerminalMux::create_debug_subscriber();

        let subscriber_id = mux.subscribe(subscriber);

        // 发送一些通知进行测试（主要测试不会panic）
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        mux.notify(MuxNotification::PaneOutput {
            pane_id: PaneId::new(1),
            data: b"test debug output".to_vec().into(),
        });
        mux.notify(MuxNotification::PaneRemoved(PaneId::new(1)));

        thread::sleep(Duration::from_millis(10));

        // 清理
        mux.unsubscribe(subscriber_id);
    }

    #[test]
    fn test_subscriber_panic_handling() {
        let mux = TerminalMux::new();
        let normal_count = Arc::new(AtomicUsize::new(0));
        let normal_clone = Arc::clone(&normal_count);

        // 添加一个会panic的订阅者
        mux.subscribe(|_| {
            panic!("Test panic in subscriber");
        });

        // 添加一个正常的订阅者
        mux.subscribe(move |_| {
            normal_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        // 发送通知，panic的订阅者应该被清理，正常的订阅者应该继续工作
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        thread::sleep(Duration::from_millis(10));

        // 再次发送通知，只有正常的订阅者应该收到
        mux.notify(MuxNotification::PaneAdded(PaneId::new(2)));
        thread::sleep(Duration::from_millis(10));

        // 正常的订阅者应该收到两次通知
        assert_eq!(normal_count.load(Ordering::Relaxed), 2);
    }
}
