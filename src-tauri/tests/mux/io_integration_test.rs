//! 集成测试 - 完整的 I/O 处理流程测试

#[cfg(test)]
mod tests {
    use crossbeam_channel::unbounded;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;

    use terminal_lib::mux::{MuxNotification, PtySize, TerminalMux};

    #[tokio::test]
    async fn test_complete_io_workflow() {
        // 创建 TerminalMux 实例
        let mux = Arc::new(TerminalMux::new());

        // 设置通知计数器
        let output_count = Arc::new(AtomicUsize::new(0));
        let removed_count = Arc::new(AtomicUsize::new(0));

        let output_count_clone = Arc::clone(&output_count);
        let removed_count_clone = Arc::clone(&removed_count);

        // 订阅通知
        let _subscriber_id = mux.subscribe(move |notification| {
            match notification {
                MuxNotification::PaneOutput { .. } => {
                    output_count_clone.fetch_add(1, Ordering::Relaxed);
                }
                MuxNotification::PaneRemoved { .. } => {
                    removed_count_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            true // 继续订阅
        });

        // 创建终端面板
        let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();

        // 写入一些命令
        mux.write_to_pane(pane_id, b"echo 'Hello, World!'\n")
            .unwrap();
        mux.write_to_pane(pane_id, b"ls\n").unwrap();

        // 等待一些输出
        thread::sleep(Duration::from_millis(500));

        // 处理跨线程通知
        mux.process_notifications();

        // 关闭面板
        mux.remove_pane(pane_id).unwrap();

        // 等待退出通知
        thread::sleep(Duration::from_millis(100));
        mux.process_notifications();

        // 验证收到了输出和退出通知
        let output_received = output_count.load(Ordering::Relaxed);
        let exit_received = removed_count.load(Ordering::Relaxed);

        println!("收到 {output_received} 个输出通知, {exit_received} 个移除通知");

        // 应该至少收到一些输出（具体数量取决于系统和命令执行）
        assert!(output_received > 0, "应该收到至少一个输出通知");
        assert_eq!(exit_received, 1, "应该收到一个移除通知");
    }

    #[tokio::test]
    async fn test_multiple_panes_io() {
        let mux = Arc::new(TerminalMux::new());

        // 创建多个面板
        let pane1 = mux.create_pane(PtySize::new(24, 80)).await.unwrap();
        let pane2 = mux.create_pane(PtySize::new(30, 100)).await.unwrap();

        // 向不同面板写入数据
        mux.write_to_pane(pane1, b"echo 'Pane 1'\n").unwrap();
        mux.write_to_pane(pane2, b"echo 'Pane 2'\n").unwrap();

        // 等待处理
        thread::sleep(Duration::from_millis(300));
        mux.process_notifications();

        // 清理
        mux.remove_pane(pane1).unwrap();
        mux.remove_pane(pane2).unwrap();

        thread::sleep(Duration::from_millis(100));
        mux.process_notifications();

        // 验证面板已被清理
        assert_eq!(mux.pane_count(), 0);
    }

    #[tokio::test]
    async fn test_io_error_handling() {
        let mux = Arc::new(TerminalMux::new());

        // 创建面板
        let pane_id = mux.create_pane(PtySize::default()).await.unwrap();

        // 获取面板引用
        let pane = mux.get_pane(pane_id).unwrap();

        // 标记面板为死亡状态（模拟错误）
        pane.mark_dead();

        // 尝试写入应该失败
        let write_result = mux.write_to_pane(pane_id, b"test\n");
        assert!(write_result.is_err(), "写入死亡面板应该失败");

        // 等待 I/O 线程检测到死亡状态并退出
        thread::sleep(Duration::from_millis(100));
        mux.process_notifications();

        // 清理
        mux.remove_pane(pane_id).unwrap();
    }

    #[test]
    fn test_batch_processing_configuration() {
        // 测试不同的批处理配置
        let (sender, _receiver) = unbounded();

        let config = terminal_lib::mux::IoConfig {
            buffer_size: 8192,
            batch_size: 512,
            flush_interval_ms: 8,
        };

        // IoHandler 需要一个 ShellIntegrationManager 实例
        let shell_mgr = std::sync::Arc::new(
            terminal_lib::shell::integration::ShellIntegrationManager::new().unwrap(),
        );
        let handler = terminal_lib::mux::IoHandler::with_config_and_mode(
            sender,
            config.clone(),
            terminal_lib::mux::IoMode::ThreadPool,
            shell_mgr,
        );

        // 验证配置被正确应用
        assert_eq!(handler.config().buffer_size, 8192);
        assert_eq!(handler.config().batch_size, 512);
        assert_eq!(handler.config().flush_interval_ms, 8);
    }

    #[tokio::test]
    async fn test_resize_during_io() {
        let mux = Arc::new(TerminalMux::new());

        // 创建面板
        let pane_id = mux.create_pane(PtySize::new(24, 80)).await.unwrap();

        // 开始一些 I/O 操作
        mux.write_to_pane(pane_id, b"echo 'Starting...'\n").unwrap();

        // 在 I/O 进行时调整大小
        let new_size = PtySize::new(30, 120);
        mux.resize_pane(pane_id, new_size).unwrap();

        // 继续 I/O 操作
        mux.write_to_pane(pane_id, b"echo 'After resize'\n")
            .unwrap();

        // 等待处理
        thread::sleep(Duration::from_millis(200));
        mux.process_notifications();

        // 验证大小已更新
        let pane = mux.get_pane(pane_id).unwrap();
        let current_size = pane.get_size();
        assert_eq!(current_size.rows, 30);
        assert_eq!(current_size.cols, 120);

        // 清理
        mux.remove_pane(pane_id).unwrap();
    }
}
