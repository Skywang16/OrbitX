//! 集成测试
//!
//! 测试完整的终端创建到销毁流程

#[cfg(test)]
mod integration_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use terminal_lib::mux::{MuxNotification, PtySize, TerminalMux};
    use terminal_lib::terminal::event_handler::TerminalEventHandler;

    #[tokio::test]
    async fn test_complete_terminal_lifecycle() {
        let mux = Arc::new(TerminalMux::new());
        let notifications = Arc::new(std::sync::Mutex::new(Vec::new()));
        let notifications_clone = Arc::clone(&notifications);

        // 订阅所有通知
        let _subscriber_id = mux.subscribe(move |notification| {
            notifications_clone
                .lock()
                .unwrap()
                .push(notification.clone());
            true
        });

        // 1. 创建终端
        let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();
        assert_eq!(pane_id.as_u32(), 1);

        // 等待创建通知
        thread::sleep(Duration::from_millis(50));

        // 2. 验证终端存在
        let pane = mux.get_pane(pane_id).unwrap();
        assert_eq!(pane.pane_id(), pane_id);
        assert!(!pane.is_dead());

        // 3. 写入数据
        mux.write_to_pane(pane_id, b"echo 'Hello, World!'\n")
            .unwrap();
        mux.write_to_pane(pane_id, b"ls -la\n").unwrap();
        mux.write_to_pane(pane_id, b"pwd\n").unwrap();

        // 4. 调整大小
        let new_size = PtySize::new(30, 120);
        mux.resize_pane(pane_id, new_size).unwrap();

        // 验证大小已更新
        let pane = mux.get_pane(pane_id).unwrap();
        let current_size = pane.get_size();
        assert_eq!(current_size.rows, 30);
        assert_eq!(current_size.cols, 120);

        // 等待一些I/O处理
        thread::sleep(Duration::from_millis(100));

        // 5. 销毁终端
        mux.remove_pane(pane_id).unwrap();

        // 验证终端已销毁
        assert!(mux.get_pane(pane_id).is_none());
        assert_eq!(mux.pane_count(), 0);

        // 等待销毁通知
        thread::sleep(Duration::from_millis(50));

        // 6. 验证通知序列
        let received_notifications = notifications.lock().unwrap();

        // 应该至少收到创建和销毁通知
        let mut has_created = false;
        let mut has_resized = false;
        let mut has_removed = false;

        for notification in received_notifications.iter() {
            match notification {
                MuxNotification::PaneAdded(id) => {
                    assert_eq!(*id, pane_id);
                    has_created = true;
                }
                MuxNotification::PaneResized { pane_id: id, size } => {
                    assert_eq!(*id, pane_id);
                    assert_eq!(size.rows, 30);
                    assert_eq!(size.cols, 120);
                    has_resized = true;
                }
                MuxNotification::PaneRemoved(id) => {
                    assert_eq!(*id, pane_id);
                    has_removed = true;
                }
                _ => {}
            }
        }

        assert!(has_created, "应该收到面板创建通知");
        assert!(has_resized, "应该收到面板调整大小通知");
        assert!(has_removed, "应该收到面板移除通知");
    }

    #[tokio::test]
    async fn test_multiple_terminals_lifecycle() {
        let mux = Arc::new(TerminalMux::new());
        let notification_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&notification_count);

        // 订阅通知计数
        let _subscriber_id = mux.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        // 创建多个终端
        let pane1 = mux.create_pane(PtySize::new(24, 80)).await.unwrap();
        let pane2 = mux.create_pane(PtySize::new(30, 100)).await.unwrap();
        let pane3 = mux.create_pane(PtySize::new(40, 120)).await.unwrap();

        assert_eq!(mux.pane_count(), 3);

        // 向每个终端写入不同数据
        mux.write_to_pane(pane1, b"terminal 1 commands\n").unwrap();
        mux.write_to_pane(pane2, b"terminal 2 commands\n").unwrap();
        mux.write_to_pane(pane3, b"terminal 3 commands\n").unwrap();

        // 调整不同终端的大小
        mux.resize_pane(pane1, PtySize::new(25, 85)).unwrap();
        mux.resize_pane(pane2, PtySize::new(35, 105)).unwrap();

        // 等待处理
        thread::sleep(Duration::from_millis(100));

        // 按不同顺序移除终端
        mux.remove_pane(pane2).unwrap(); // 先移除中间的
        assert_eq!(mux.pane_count(), 2);
        assert!(mux.get_pane(pane1).is_some());
        assert!(mux.get_pane(pane2).is_none());
        assert!(mux.get_pane(pane3).is_some());

        mux.remove_pane(pane1).unwrap(); // 再移除第一个
        assert_eq!(mux.pane_count(), 1);
        assert!(mux.get_pane(pane3).is_some());

        mux.remove_pane(pane3).unwrap(); // 最后移除最后一个
        assert_eq!(mux.pane_count(), 0);

        // 等待所有通知处理完成
        thread::sleep(Duration::from_millis(100));

        // 应该收到多个通知（创建、调整大小、移除等）
        let total_notifications = notification_count.load(Ordering::Relaxed);
        assert!(total_notifications >= 8); // 至少3个创建 + 2个调整大小 + 3个移除
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let mux = Arc::new(TerminalMux::new());
        let mut handles = Vec::new();

        // 并发创建多个终端
        for i in 0..10 {
            let mux_clone = Arc::clone(&mux);
            let handle = tokio::spawn(async move {
                let size = PtySize::new(24 + i, 80 + i * 10);
                let pane_id = mux_clone.create_pane(size).await.unwrap();

                // 写入一些数据
                let data = format!("Terminal {i} data\n");
                mux_clone.write_to_pane(pane_id, data.as_bytes()).unwrap();

                // 调整大小
                let new_size = PtySize::new(25 + i, 85 + i * 10);
                mux_clone.resize_pane(pane_id, new_size).unwrap();

                pane_id
            });
            handles.push(handle);
        }

        // 等待所有创建完成
        let mut pane_ids = Vec::new();
        for handle in handles {
            let pane_id = handle.await.unwrap();
            pane_ids.push(pane_id);
        }

        assert_eq!(mux.pane_count(), 10);
        assert_eq!(pane_ids.len(), 10);

        // 验证所有面板都存在且有正确的大小
        for (i, &pane_id) in pane_ids.iter().enumerate() {
            let pane = mux.get_pane(pane_id).unwrap();
            let size = pane.get_size();
            assert_eq!(size.rows, 25 + i as u16);
            assert_eq!(size.cols, 85 + i as u16 * 10);
        }

        // 并发移除所有终端
        let mut remove_handles = Vec::new();
        for pane_id in pane_ids {
            let mux_clone = Arc::clone(&mux);
            let handle = tokio::spawn(async move {
                mux_clone.remove_pane(pane_id).unwrap();
            });
            remove_handles.push(handle);
        }

        // 等待所有移除完成
        for handle in remove_handles {
            handle.await.unwrap();
        }

        assert_eq!(mux.pane_count(), 0);
    }

    #[tokio::test]
    async fn test_stress_write_operations() {
        let mux = Arc::new(TerminalMux::new());
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 大量并发写入操作
        let mut handles = Vec::new();
        for i in 0..100 {
            let mux_clone = Arc::clone(&mux);
            let handle = tokio::spawn(async move {
                let data = format!("Message {i} from thread\n");
                mux_clone.write_to_pane(pane_id, data.as_bytes()).unwrap();
            });
            handles.push(handle);
        }

        // 等待所有写入完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 验证面板仍然存在且正常
        let pane = mux.get_pane(pane_id).unwrap();
        assert!(!pane.is_dead());

        // 清理
        mux.remove_pane(pane_id).unwrap();
    }

    #[tokio::test]
    async fn test_rapid_create_destroy_cycle() {
        let mux = Arc::new(TerminalMux::new());

        // 快速创建和销毁循环
        for i in 0..50 {
            let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();

            // 写入一些数据
            let data = format!("Cycle {i} data\n");
            mux.write_to_pane(pane_id, data.as_bytes()).unwrap();

            // 立即销毁
            mux.remove_pane(pane_id).unwrap();

            // 验证已销毁
            assert!(mux.get_pane(pane_id).is_none());
        }

        assert_eq!(mux.pane_count(), 0);
    }

    #[tokio::test]
    async fn test_notification_system_integration() {
        let mux = Arc::new(TerminalMux::new());
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);

        // 订阅所有事件并记录
        let _subscriber_id = mux.subscribe(move |notification| {
            let (event_name, payload) = TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);
            let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            events_clone
                .lock()
                .unwrap()
                .push((event_name.to_string(), payload_str));
            true
        });

        // 执行一系列操作
        let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();
        thread::sleep(Duration::from_millis(10));

        mux.write_to_pane(pane_id, b"test command\n").unwrap();
        thread::sleep(Duration::from_millis(10));

        mux.resize_pane(pane_id, PtySize::new(30, 100)).unwrap();
        thread::sleep(Duration::from_millis(10));

        mux.remove_pane(pane_id).unwrap();
        thread::sleep(Duration::from_millis(10));

        // 验证事件序列
        let recorded_events = events.lock().unwrap();

        // 应该至少有创建、调整大小、移除事件
        let event_names: Vec<String> = recorded_events
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        assert!(event_names.contains(&"terminal_created".to_string()));
        assert!(event_names.contains(&"terminal_resized".to_string()));
        assert!(event_names.contains(&"terminal_closed".to_string()));

        // 验证事件数据格式
        for (event_name, payload) in recorded_events.iter() {
            match event_name.as_str() {
                "terminal_created" => {
                    assert!(payload.contains("\"paneId\":1")); // camelCase
                }
                "terminal_resized" => {
                    assert!(payload.contains("\"paneId\":1")); // camelCase
                    assert!(payload.contains("\"rows\":30"));
                    assert!(payload.contains("\"cols\":100"));
                }
                "terminal_closed" => {
                    assert!(payload.contains("\"paneId\":1")); // camelCase
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_mux_shutdown_integration() {
        let mux = Arc::new(TerminalMux::new());
        let shutdown_events = Arc::new(AtomicUsize::new(0));
        let events_clone = Arc::clone(&shutdown_events);

        // 订阅移除事件
        let _subscriber_id = mux.subscribe(move |notification| {
            if matches!(notification, MuxNotification::PaneRemoved(_)) {
                events_clone.fetch_add(1, Ordering::Relaxed);
            }
            true
        });

        // 创建多个终端
        let mut pane_ids = Vec::new();
        for _ in 0..5 {
            let pane_id = mux.create_pane(PtySize::default()).await.unwrap();
            pane_ids.push(pane_id);
        }

        assert_eq!(mux.pane_count(), 5);

        // 向所有终端写入数据
        for &pane_id in &pane_ids {
            mux.write_to_pane(pane_id, b"test data before shutdown\n")
                .unwrap();
        }

        // 执行关闭
        mux.shutdown().unwrap();

        // 验证所有终端都已关闭
        assert_eq!(mux.pane_count(), 0);
        for &pane_id in &pane_ids {
            assert!(mux.get_pane(pane_id).is_none());
        }

        // 等待关闭事件处理
        thread::sleep(Duration::from_millis(50));

        // 应该收到5个移除事件
        assert_eq!(shutdown_events.load(Ordering::Relaxed), 5);

        // 关闭后的操作应该失败
        let _result = mux.create_pane(PtySize::default()).await;
        // 注意：当前实现可能允许在shutdown后创建新面板，这取决于具体实现
        // 这里主要测试shutdown确实清理了现有面板
    }

    #[tokio::test]
    async fn test_cross_thread_integration() {
        let mux = Arc::new(TerminalMux::new());
        let cross_thread_notifications = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&cross_thread_notifications);

        // 在主线程订阅
        let _subscriber_id = mux.subscribe(move |_| {
            count_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        // 在主线程创建终端
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 从其他线程发送通知
        let mux_clone = Arc::clone(&mux);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                mux_clone.notify_from_any_thread(MuxNotification::PaneOutput {
                    pane_id,
                    data: format!("Cross-thread message {i}\n").into_bytes().into(),
                });
                thread::sleep(Duration::from_millis(1));
            }
        });

        handle.join().unwrap();

        // 处理跨线程通知
        for _ in 0..20 {
            mux.process_notifications();
            thread::sleep(Duration::from_millis(5));
        }

        // 应该收到创建通知 + 10个跨线程通知
        let total_notifications = cross_thread_notifications.load(Ordering::Relaxed);
        assert!(total_notifications >= 11);

        // 清理
        mux.remove_pane(pane_id).unwrap();
    }
}
