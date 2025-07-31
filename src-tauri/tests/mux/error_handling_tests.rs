//! 错误处理测试
//!
//! 测试各种异常情况的处理

#[cfg(test)]
mod error_handling_tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use terminal_lib::mux::{MuxNotification, PaneId, PtySize, TerminalMux};

    #[tokio::test]
    async fn test_pane_not_found_errors() {
        let mux = TerminalMux::new();
        let nonexistent_pane = PaneId::new(999);

        // 测试获取不存在的面板
        assert!(mux.get_pane(nonexistent_pane).is_none());

        // 测试写入不存在的面板
        let result = mux.write_to_pane(nonexistent_pane, b"test");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("不存在") || error_msg.contains("not found"));

        // 测试调整不存在面板的大小
        let result = mux.resize_pane(nonexistent_pane, PtySize::default());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("不存在") || error_msg.contains("not found"));

        // 测试移除不存在的面板
        let result = mux.remove_pane(nonexistent_pane);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("不存在") || error_msg.contains("not found"));
    }

    #[tokio::test]
    async fn test_dead_pane_operations() {
        let mux = TerminalMux::new();
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 获取面板并标记为死亡
        let pane = mux.get_pane(pane_id).unwrap();
        pane.mark_dead();
        assert!(pane.is_dead());

        // 对死亡面板的操作应该失败
        let result = mux.write_to_pane(pane_id, b"test");
        assert!(result.is_err());

        let result = mux.resize_pane(pane_id, PtySize::new(30, 100));
        assert!(result.is_err());

        // 但是移除死亡面板应该成功
        let result = mux.remove_pane(pane_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_handling_with_anyhow() {
        // 测试 anyhow 错误处理
        let error = anyhow::anyhow!("test error message");
        assert_eq!(error.to_string(), "test error message");

        // 测试错误链
        let root_cause = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = anyhow::Error::from(root_cause).context("failed to read file");
        assert!(error.to_string().contains("failed to read file"));
        assert!(error.to_string().contains("file not found"));
    }

    #[test]
    fn test_anyhow_error_context() {
        // 测试 anyhow 错误上下文
        let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("base error"))
            .context("operation failed")
            .context("high level operation failed");

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("high level operation failed"));
        assert!(error_msg.contains("operation failed"));
        assert!(error_msg.contains("base error"));
    }

    #[test]
    fn test_subscriber_error_handling() {
        let mux = TerminalMux::new();
        let panic_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let normal_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let panic_clone = Arc::clone(&panic_count);
        let normal_clone = Arc::clone(&normal_count);

        // 添加会panic的订阅者
        mux.subscribe(move |_| {
            panic_clone.fetch_add(1, Ordering::Relaxed);
            panic!("Intentional panic in subscriber");
        });

        // 添加正常的订阅者
        mux.subscribe(move |_| {
            normal_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        // 发送通知
        mux.notify(MuxNotification::PaneAdded(PaneId::new(1)));
        thread::sleep(Duration::from_millis(10));

        // panic的订阅者应该被调用一次然后被清理
        assert_eq!(panic_count.load(Ordering::Relaxed), 1);
        // 正常的订阅者应该正常工作
        assert_eq!(normal_count.load(Ordering::Relaxed), 1);

        // 再次发送通知，panic的订阅者应该已经被清理
        mux.notify(MuxNotification::PaneAdded(PaneId::new(2)));
        thread::sleep(Duration::from_millis(10));

        // panic的订阅者不应该再被调用
        assert_eq!(panic_count.load(Ordering::Relaxed), 1);
        // 正常的订阅者应该继续工作
        assert_eq!(normal_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_cross_thread_notification_errors() {
        let mux = Arc::new(TerminalMux::new());
        let error_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let error_clone = Arc::clone(&error_count);
        let success_clone = Arc::clone(&success_count);

        // 订阅通知
        let _subscriber_id = mux.subscribe(move |notification| {
            if let MuxNotification::PaneOutput { data, .. } = notification {
                if data.starts_with(b"ERROR") {
                    error_clone.fetch_add(1, Ordering::Relaxed);
                } else {
                    success_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            true
        });

        // 从多个线程发送通知，包括一些"错误"消息
        let mut handles = Vec::new();
        for i in 0..10 {
            let mux_clone = Arc::clone(&mux);
            let handle = thread::spawn(move || {
                let data = if i % 3 == 0 {
                    format!("ERROR: Message {i}").into_bytes()
                } else {
                    format!("SUCCESS: Message {i}").into_bytes()
                };

                mux_clone.notify_from_any_thread(MuxNotification::PaneOutput {
                    pane_id: PaneId::new(1),
                    data,
                });
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 处理跨线程通知
        for _ in 0..20 {
            mux.process_notifications();
            thread::sleep(Duration::from_millis(5));
        }

        // 验证错误和成功消息都被正确处理
        let total_errors = error_count.load(Ordering::Relaxed);
        let total_success = success_count.load(Ordering::Relaxed);

        assert_eq!(total_errors + total_success, 10);
        assert!(total_errors > 0);
        assert!(total_success > 0);
    }

    #[tokio::test]
    async fn test_concurrent_error_scenarios() {
        let mux = Arc::new(TerminalMux::new());

        // 创建一个终端
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 并发执行可能失败的操作
        let mut handles = Vec::new();

        // 一些线程尝试写入
        for i in 0..5 {
            let mux_clone = Arc::clone(&mux);
            let handle = tokio::spawn(async move {
                let data = format!("Concurrent write {i}\n");
                mux_clone.write_to_pane(pane_id, data.as_bytes())
            });
            handles.push(handle);
        }

        // 一些线程尝试调整大小
        for i in 0..3 {
            let mux_clone = Arc::clone(&mux);
            let handle = tokio::spawn(async move {
                let size = PtySize::new(24 + i, 80 + i * 10);
                mux_clone.resize_pane(pane_id, size)
            });
            handles.push(handle);
        }

        // 一个线程尝试移除面板
        let mux_clone = Arc::clone(&mux);
        let remove_handle = tokio::spawn(async move {
            thread::sleep(Duration::from_millis(50)); // 稍后执行
            mux_clone.remove_pane(pane_id)
        });
        handles.push(remove_handle);

        // 等待所有操作完成并检查结果
        let mut success_count = 0;
        let mut _error_count = 0;

        for handle in handles {
            match handle.await.unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => _error_count += 1,
            }
        }

        // 应该有一些成功和一些失败（因为面板被移除后的操作会失败）
        assert!(success_count > 0);
        // 注意：_error_count可能为0，取决于操作的时序

        // 验证面板最终被移除
        assert!(mux.get_pane(pane_id).is_none());
    }

    #[tokio::test]
    async fn test_resource_exhaustion_simulation() {
        let mux = TerminalMux::new();
        let mut pane_ids = Vec::new();

        // 尝试创建大量终端（模拟资源耗尽）
        for i in 0..100 {
            match mux.create_pane(PtySize::new(24, 80)).await {
                Ok(pane_id) => {
                    pane_ids.push(pane_id);

                    // 向每个终端写入数据
                    let data = format!("Terminal {i} data\n");
                    let _ = mux.write_to_pane(pane_id, data.as_bytes());
                }
                Err(e) => {
                    // 如果创建失败，记录错误但继续
                    eprintln!("Failed to create pane {i}: {e}");
                    break;
                }
            }
        }

        println!("Successfully created {} terminals", pane_ids.len());

        // 清理所有创建的终端
        for pane_id in pane_ids {
            let _ = mux.remove_pane(pane_id);
        }

        assert_eq!(mux.pane_count(), 0);
    }

    #[test]
    fn test_notification_channel_errors() {
        let mux = Arc::new(TerminalMux::new());
        let received_notifications = Arc::new(std::sync::Mutex::new(Vec::new()));
        let notifications_clone = Arc::clone(&received_notifications);

        // 订阅通知
        let _subscriber_id = mux.subscribe(move |notification| {
            notifications_clone.lock().unwrap().push(notification);
            true
        });

        // 从多个线程同时发送大量通知
        let mut handles = Vec::new();
        for thread_id in 0..10 {
            let mux_clone = Arc::clone(&mux);
            let handle = thread::spawn(move || {
                for i in 0..50 {
                    let notification = MuxNotification::PaneOutput {
                        pane_id: PaneId::new(thread_id),
                        data: format!("Thread {thread_id} message {i}").into_bytes(),
                    };
                    mux_clone.notify_from_any_thread(notification);
                }
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 处理所有通知
        for _ in 0..100 {
            mux.process_notifications();
            thread::sleep(Duration::from_millis(1));
        }

        // 验证收到了大量通知
        let notifications = received_notifications.lock().unwrap();
        assert!(notifications.len() > 0);
        println!("Received {} notifications", notifications.len());
    }

    #[test]
    fn test_anyhow_error_formatting() {
        // 测试 anyhow 错误消息的格式化
        let errors = vec![
            anyhow::anyhow!("面板 {:?} 不存在", PaneId::new(42)),
            anyhow::anyhow!("面板 {:?} 已存在", PaneId::new(123)),
            anyhow::anyhow!("PTY operation failed"),
            anyhow::anyhow!("Internal system error"),
            anyhow::anyhow!("Invalid configuration"),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            println!("Error: {error_string}");
        }
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let mux = TerminalMux::new();

        // 创建一个终端
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 模拟部分功能失败的情况
        let pane = mux.get_pane(pane_id).unwrap();

        // 正常操作应该成功
        assert!(mux.write_to_pane(pane_id, b"normal operation\n").is_ok());
        assert!(mux.resize_pane(pane_id, PtySize::new(30, 100)).is_ok());

        // 标记面板为死亡，模拟PTY失败
        pane.mark_dead();

        // 后续操作应该优雅地失败
        assert!(mux.write_to_pane(pane_id, b"should fail\n").is_err());
        assert!(mux.resize_pane(pane_id, PtySize::new(40, 120)).is_err());

        // 但是查询操作仍然应该工作
        assert!(mux.get_pane(pane_id).is_some());
        assert_eq!(mux.pane_count(), 1);

        // 清理操作应该成功
        assert!(mux.remove_pane(pane_id).is_ok());
        assert_eq!(mux.pane_count(), 0);
    }
}
