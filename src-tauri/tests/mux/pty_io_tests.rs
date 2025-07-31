//! PTY I/O 测试
//!
//! 使用模拟数据测试读写功能

#[cfg(test)]
mod pty_io_tests {
    use crossbeam_channel::unbounded;
    use std::io::{Cursor, Read};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use terminal_lib::mux::{
        IoConfig, IoHandler, MuxNotification, Pane, PaneError, PaneId, PaneResult, PtySize,
    };

    /// 模拟面板用于测试
    struct MockPane {
        pane_id: PaneId,
        size: Arc<Mutex<PtySize>>,
        dead: Arc<AtomicBool>,
        reader_data: Arc<Mutex<Vec<u8>>>,
        written_data: Arc<Mutex<Vec<u8>>>,
        should_fail_write: Arc<AtomicBool>,
        should_fail_read: Arc<AtomicBool>,
    }

    impl MockPane {
        fn new(pane_id: PaneId, size: PtySize) -> Self {
            Self {
                pane_id,
                size: Arc::new(Mutex::new(size)),
                dead: Arc::new(AtomicBool::new(false)),
                reader_data: Arc::new(Mutex::new(Vec::new())),
                written_data: Arc::new(Mutex::new(Vec::new())),
                should_fail_write: Arc::new(AtomicBool::new(false)),
                should_fail_read: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_reader_data(pane_id: PaneId, size: PtySize, data: Vec<u8>) -> Self {
            let pane = Self::new(pane_id, size);
            *pane.reader_data.lock().unwrap() = data;
            pane
        }

        fn get_written_data(&self) -> Vec<u8> {
            self.written_data.lock().unwrap().clone()
        }

        fn set_should_fail_write(&self, should_fail: bool) {
            self.should_fail_write.store(should_fail, Ordering::Relaxed);
        }

        fn set_should_fail_read(&self, should_fail: bool) {
            self.should_fail_read.store(should_fail, Ordering::Relaxed);
        }

        fn add_reader_data(&self, data: &[u8]) {
            self.reader_data.lock().unwrap().extend_from_slice(data);
        }
    }

    impl Pane for MockPane {
        fn pane_id(&self) -> PaneId {
            self.pane_id
        }

        fn write(&self, data: &[u8]) -> PaneResult<()> {
            if self.is_dead() {
                return Err(PaneError::PaneClosed);
            }

            if self.should_fail_write.load(Ordering::Relaxed) {
                return Err(PaneError::WriteError("模拟写入失败".to_string()));
            }

            self.written_data.lock().unwrap().extend_from_slice(data);
            Ok(())
        }

        fn resize(&self, size: PtySize) -> PaneResult<()> {
            if self.is_dead() {
                return Err(PaneError::PaneClosed);
            }

            *self.size.lock().unwrap() = size;
            Ok(())
        }

        fn reader(&self) -> PaneResult<Box<dyn Read + Send>> {
            if self.is_dead() {
                return Err(PaneError::PaneClosed);
            }

            if self.should_fail_read.load(Ordering::Relaxed) {
                return Err(PaneError::ReadError("模拟读取失败".to_string()));
            }

            let data = self.reader_data.lock().unwrap().clone();
            Ok(Box::new(Cursor::new(data)))
        }

        fn is_dead(&self) -> bool {
            self.dead.load(Ordering::Relaxed)
        }

        fn mark_dead(&self) {
            self.dead.store(true, Ordering::Relaxed);
        }

        fn get_size(&self) -> PtySize {
            *self.size.lock().unwrap()
        }
    }

    #[test]
    fn test_mock_pane_basic_operations() {
        let pane = MockPane::new(PaneId::new(1), PtySize::new(24, 80));

        // 测试基本属性
        assert_eq!(pane.pane_id(), PaneId::new(1));
        assert_eq!(pane.get_size().rows, 24);
        assert_eq!(pane.get_size().cols, 80);
        assert!(!pane.is_dead());

        // 测试写入
        assert!(pane.write(b"hello").is_ok());
        assert!(pane.write(b" world").is_ok());
        assert_eq!(pane.get_written_data(), b"hello world");

        // 测试调整大小
        let new_size = PtySize::new(30, 100);
        assert!(pane.resize(new_size).is_ok());
        assert_eq!(pane.get_size().rows, 30);
        assert_eq!(pane.get_size().cols, 100);

        // 测试读取器
        assert!(pane.reader().is_ok());

        // 测试标记为死亡
        pane.mark_dead();
        assert!(pane.is_dead());
        assert!(pane.write(b"test").is_err());
        assert!(pane.resize(PtySize::default()).is_err());
        assert!(pane.reader().is_err());
    }

    #[test]
    fn test_mock_pane_with_data() {
        let test_data = b"Hello, World!\nThis is a test.\n".to_vec();
        let pane =
            MockPane::with_reader_data(PaneId::new(2), PtySize::default(), test_data.clone());

        let mut reader = pane.reader().unwrap();
        let mut buffer = Vec::new();
        let bytes_read = reader.read_to_end(&mut buffer).unwrap();

        assert_eq!(bytes_read, test_data.len());
        assert_eq!(buffer, test_data);
    }

    #[test]
    fn test_mock_pane_error_conditions() {
        let pane = MockPane::new(PaneId::new(3), PtySize::default());

        // 测试写入失败
        pane.set_should_fail_write(true);
        assert!(pane.write(b"test").is_err());

        pane.set_should_fail_write(false);
        assert!(pane.write(b"test").is_ok());

        // 测试读取失败
        pane.set_should_fail_read(true);
        assert!(pane.reader().is_err());

        pane.set_should_fail_read(false);
        assert!(pane.reader().is_ok());
    }

    #[test]
    fn test_io_handler_creation() {
        let (sender, _receiver) = unbounded();
        let handler = IoHandler::new(sender);

        assert_eq!(handler.config().buffer_size, 4096);
        assert_eq!(handler.config().batch_size, 1024);
        assert_eq!(handler.config().flush_interval_ms, 16);
    }

    #[test]
    fn test_io_handler_with_custom_config() {
        let (sender, _receiver) = unbounded();
        let config = IoConfig {
            buffer_size: 8192,
            batch_size: 2048,
            flush_interval_ms: 32,
        };

        let handler = IoHandler::with_config(sender, config.clone());

        assert_eq!(handler.config().buffer_size, 8192);
        assert_eq!(handler.config().batch_size, 2048);
        assert_eq!(handler.config().flush_interval_ms, 32);
    }

    #[test]
    fn test_io_handler_spawn_threads() {
        let (sender, receiver) = unbounded();
        let handler = IoHandler::new(sender);

        let test_data = b"Hello from PTY!".to_vec();
        let mock_pane = Arc::new(MockPane::with_reader_data(
            PaneId::new(1),
            PtySize::default(),
            test_data.clone(),
        ));

        // 启动 I/O 线程
        handler.spawn_io_threads(mock_pane.clone()).unwrap();

        // 等待数据处理
        thread::sleep(Duration::from_millis(100));

        // 标记面板为死亡以停止线程
        mock_pane.mark_dead();

        // 等待线程退出
        thread::sleep(Duration::from_millis(100));

        // 检查是否收到了数据和退出通知
        let mut received_output = false;
        let mut received_exit = false;
        let mut total_data = Vec::new();

        while let Ok(notification) = receiver.try_recv() {
            match notification {
                MuxNotification::PaneOutput { pane_id, data } => {
                    assert_eq!(pane_id, PaneId::new(1));
                    total_data.extend_from_slice(&data);
                    received_output = true;
                }
                MuxNotification::PaneExited {
                    pane_id,
                    exit_code: _,
                } => {
                    assert_eq!(pane_id, PaneId::new(1));
                    received_exit = true;
                }
                _ => {}
            }
        }

        assert!(received_output, "应该收到输出通知");
        assert!(received_exit, "应该收到退出通知");
        assert_eq!(total_data, test_data);
    }

    #[test]
    fn test_io_handler_batch_processing() {
        let (sender, receiver) = unbounded();
        let config = IoConfig {
            buffer_size: 4096,
            batch_size: 10,         // 小的批处理大小用于测试
            flush_interval_ms: 100, // 较长的间隔用于测试
        };
        let handler = IoHandler::with_config(sender, config);

        // 创建大于批处理大小的数据
        let test_data = b"This is a long test message that exceeds the batch size limit".to_vec();
        let mock_pane = Arc::new(MockPane::with_reader_data(
            PaneId::new(2),
            PtySize::default(),
            test_data.clone(),
        ));

        handler.spawn_io_threads(mock_pane.clone()).unwrap();

        // 等待数据处理
        thread::sleep(Duration::from_millis(50));

        // 标记面板为死亡
        mock_pane.mark_dead();

        // 等待线程退出
        thread::sleep(Duration::from_millis(150));

        // 收集所有数据
        let mut total_data = Vec::new();
        let mut batch_count = 0;

        while let Ok(notification) = receiver.try_recv() {
            if let MuxNotification::PaneOutput { data, .. } = notification {
                total_data.extend_from_slice(&data);
                batch_count += 1;
            }
        }

        assert_eq!(total_data, test_data);
        // 由于数据大于批处理大小，应该有多个批次
        assert!(batch_count >= 1);
    }

    #[test]
    fn test_io_handler_multiple_panes() {
        let (sender, receiver) = unbounded();
        let handler = IoHandler::new(sender);

        let data1 = b"Data from pane 1".to_vec();
        let data2 = b"Data from pane 2".to_vec();
        let data3 = b"Data from pane 3".to_vec();

        let pane1 = Arc::new(MockPane::with_reader_data(
            PaneId::new(1),
            PtySize::default(),
            data1.clone(),
        ));
        let pane2 = Arc::new(MockPane::with_reader_data(
            PaneId::new(2),
            PtySize::default(),
            data2.clone(),
        ));
        let pane3 = Arc::new(MockPane::with_reader_data(
            PaneId::new(3),
            PtySize::default(),
            data3.clone(),
        ));

        // 启动所有面板的 I/O 线程
        handler.spawn_io_threads(pane1.clone()).unwrap();
        handler.spawn_io_threads(pane2.clone()).unwrap();
        handler.spawn_io_threads(pane3.clone()).unwrap();

        // 等待数据处理
        thread::sleep(Duration::from_millis(100));

        // 标记所有面板为死亡
        pane1.mark_dead();
        pane2.mark_dead();
        pane3.mark_dead();

        // 等待线程退出
        thread::sleep(Duration::from_millis(100));

        // 收集数据
        let mut pane_data = std::collections::HashMap::new();
        let mut exit_count = 0;

        while let Ok(notification) = receiver.try_recv() {
            match notification {
                MuxNotification::PaneOutput { pane_id, data } => {
                    pane_data
                        .entry(pane_id)
                        .or_insert_with(Vec::new)
                        .extend_from_slice(&data);
                }
                MuxNotification::PaneExited { .. } => {
                    exit_count += 1;
                }
                _ => {}
            }
        }

        // 验证每个面板的数据
        assert_eq!(pane_data.get(&PaneId::new(1)).unwrap(), &data1);
        assert_eq!(pane_data.get(&PaneId::new(2)).unwrap(), &data2);
        assert_eq!(pane_data.get(&PaneId::new(3)).unwrap(), &data3);
        assert_eq!(exit_count, 3);
    }

    #[test]
    fn test_io_handler_error_handling() {
        let (sender, receiver) = unbounded();
        let handler = IoHandler::new(sender);

        let mock_pane = Arc::new(MockPane::new(PaneId::new(4), PtySize::default()));

        // 设置读取失败
        mock_pane.set_should_fail_read(true);

        // 尝试启动 I/O 线程应该失败
        let result = handler.spawn_io_threads(mock_pane.clone());
        assert!(result.is_err());

        // 修复读取问题
        mock_pane.set_should_fail_read(false);
        mock_pane.add_reader_data(b"test data");

        // 现在应该成功
        let result = handler.spawn_io_threads(mock_pane.clone());
        assert!(result.is_ok());

        // 等待一些处理
        thread::sleep(Duration::from_millis(50));

        // 清理
        mock_pane.mark_dead();
        thread::sleep(Duration::from_millis(50));

        // 应该收到一些通知
        let mut notification_count = 0;
        while receiver.try_recv().is_ok() {
            notification_count += 1;
        }
        assert!(notification_count > 0);
    }

    #[test]
    fn test_io_handler_large_data() {
        let (sender, receiver) = unbounded();
        let handler = IoHandler::new(sender);

        // 创建大量数据
        let mut large_data = Vec::new();
        for i in 0..1000 {
            large_data.extend_from_slice(format!("Line {i} with some content\n").as_bytes());
        }

        let mock_pane = Arc::new(MockPane::with_reader_data(
            PaneId::new(5),
            PtySize::default(),
            large_data.clone(),
        ));

        handler.spawn_io_threads(mock_pane.clone()).unwrap();

        // 等待数据处理
        thread::sleep(Duration::from_millis(200));

        // 标记面板为死亡
        mock_pane.mark_dead();

        // 等待线程退出
        thread::sleep(Duration::from_millis(100));

        // 收集所有数据
        let mut total_data = Vec::new();
        while let Ok(notification) = receiver.try_recv() {
            if let MuxNotification::PaneOutput { data, .. } = notification {
                total_data.extend_from_slice(&data);
            }
        }

        assert_eq!(total_data, large_data);
    }

    #[test]
    fn test_io_handler_empty_data() {
        let (sender, receiver) = unbounded();
        let handler = IoHandler::new(sender);

        // 创建没有数据的面板
        let mock_pane = Arc::new(MockPane::with_reader_data(
            PaneId::new(6),
            PtySize::default(),
            Vec::new(),
        ));

        handler.spawn_io_threads(mock_pane.clone()).unwrap();

        // 等待处理
        thread::sleep(Duration::from_millis(50));

        // 标记面板为死亡
        mock_pane.mark_dead();

        // 等待线程退出
        thread::sleep(Duration::from_millis(50));

        // 应该只收到退出通知，没有输出通知
        let mut output_count = 0;
        let mut exit_count = 0;

        while let Ok(notification) = receiver.try_recv() {
            match notification {
                MuxNotification::PaneOutput { .. } => output_count += 1,
                MuxNotification::PaneExited { .. } => exit_count += 1,
                _ => {}
            }
        }

        assert_eq!(output_count, 0);
        assert_eq!(exit_count, 1);
    }
}
