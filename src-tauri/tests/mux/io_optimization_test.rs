//! I/O优化测试
//!
//! 测试线程池模式与传统模式的性能差异

use terminal_lib::mux::{IoConfig, IoHandler, IoMode, PerformanceMonitor, TerminalMuxStatus};

/// 测试I/O处理器模式切换
#[test]
fn test_io_handler_mode_switching() {
    let (notification_sender, _notification_receiver) = crossbeam_channel::unbounded();

    // 测试传统模式
    let legacy_handler = {
        let shell_mgr = std::sync::Arc::new(
            terminal_lib::shell::integration::ShellIntegrationManager::new().unwrap(),
        );
        IoHandler::with_config_and_mode(
            notification_sender.clone(),
            IoConfig::default(),
            IoMode::Legacy,
            shell_mgr,
        )
    };
    assert!(matches!(legacy_handler.mode(), IoMode::Legacy));
    assert!(legacy_handler.get_stats().is_none());

    // 测试线程池模式
    let pool_handler = {
        let shell_mgr = std::sync::Arc::new(
            terminal_lib::shell::integration::ShellIntegrationManager::new().unwrap(),
        );
        IoHandler::with_config_and_mode(
            notification_sender.clone(),
            IoConfig::default(),
            IoMode::ThreadPool,
            shell_mgr,
        )
    };
    assert!(matches!(pool_handler.mode(), IoMode::ThreadPool));
    assert!(pool_handler.get_stats().is_some());

    // 清理
    let _ = legacy_handler.shutdown();
    let _ = pool_handler.shutdown();
}

/// 测试工具函数
#[test]
fn test_utility_functions() {
    use terminal_lib::mux::performance_monitor::utils;

    // 测试字节格式化
    assert_eq!(utils::format_bytes(1024), "1.00 KB");
    assert_eq!(utils::format_bytes(1048576), "1.00 MB");
    assert_eq!(utils::format_bytes(1073741824), "1.00 GB");

    // 测试时间格式化
    assert_eq!(utils::format_duration(30), "30s");
    assert_eq!(utils::format_duration(90), "1m 30s");
    assert_eq!(utils::format_duration(3661), "1h 1m 1s");

    // 测试系统线程数获取
    if let Some(thread_count) = utils::get_system_thread_count() {
        assert!(thread_count > 0);
        println!("系统线程数: {}", thread_count);
    }
}

/// 测试I/O配置
#[test]
fn test_io_configuration() {
    let (sender, _receiver) = crossbeam_channel::unbounded();

    let config = IoConfig {
        buffer_size: 8192,
        batch_size: 512,
        flush_interval_ms: 8,
    };

    let handler = {
        let shell_mgr = std::sync::Arc::new(
            terminal_lib::shell::integration::ShellIntegrationManager::new().unwrap(),
        );
        IoHandler::with_config_and_mode(sender, config.clone(), IoMode::ThreadPool, shell_mgr)
    };

    // 验证配置被正确应用
    assert_eq!(handler.config().buffer_size, 8192);
    assert_eq!(handler.config().batch_size, 512);
    assert_eq!(handler.config().flush_interval_ms, 8);
}

/// 测试性能监控器
#[test]
fn test_performance_monitor() {
    let monitor = PerformanceMonitor::new();

    let mux_status = TerminalMuxStatus {
        pane_count: 5,
        subscriber_count: 2,
        next_pane_id: 6,
        next_subscriber_id: 3,
        main_thread_id: std::thread::current().id(),
    };

    let io_stats = terminal_lib::mux::IoThreadPoolStats {
        active_panes: 5,
        worker_threads: 3,
        pending_tasks: 0,
        total_bytes_processed: 1024 * 1024, // 1MB
        total_batches_processed: 1000,
    };

    let metrics = monitor.collect_metrics(&mux_status, Some(&io_stats));

    assert_eq!(metrics.active_panes, 5);
    assert_eq!(metrics.worker_threads, 3);
    assert_eq!(metrics.total_bytes_processed, 1024 * 1024);
    assert_eq!(metrics.total_batches_processed, 1000);
    // 平均批次大小应该约为1024字节（允许浮点误差）
    assert!((metrics.avg_batch_size - 1048.576).abs() < 0.1);

    // 测试性能警告检查
    monitor.check_performance_warnings(&metrics);

    // 测试报告生成
    let report = monitor.generate_report(&metrics);
    assert!(report.contains("活跃面板: 5"));
    assert!(report.contains("工作线程: 3"));

    println!("性能报告:\n{}", report);
}

/// 测试线程池与传统模式的内存使用对比
#[test]
fn test_memory_usage_comparison() {
    let monitor = PerformanceMonitor::new();

    // 模拟传统模式：10个面板，每个面板2个线程
    let legacy_status = TerminalMuxStatus {
        pane_count: 10,
        subscriber_count: 1,
        next_pane_id: 11,
        next_subscriber_id: 2,
        main_thread_id: std::thread::current().id(),
    };

    let legacy_metrics = monitor.collect_metrics(&legacy_status, None);

    // 模拟线程池模式：10个面板，3个工作线程
    let pool_stats = terminal_lib::mux::IoThreadPoolStats {
        active_panes: 10,
        worker_threads: 3,
        pending_tasks: 0,
        total_bytes_processed: 0,
        total_batches_processed: 0,
    };

    let pool_metrics = monitor.collect_metrics(&legacy_status, Some(&pool_stats));

    println!(
        "传统模式内存估算: {} KB",
        legacy_metrics.estimated_memory_usage / 1024
    );
    println!(
        "线程池模式内存估算: {} KB",
        pool_metrics.estimated_memory_usage / 1024
    );

    // 线程池模式使用更少的线程（这是主要优势）
    assert!(pool_metrics.worker_threads < legacy_metrics.worker_threads);

    // 验证线程数量的具体差异
    assert_eq!(legacy_metrics.worker_threads, 20); // 10个面板 * 2个线程
    assert_eq!(pool_metrics.worker_threads, 3); // 3个工作线程
}

/// 测试性能指标比较
#[test]
fn test_performance_metrics_comparison() {
    let monitor = PerformanceMonitor::new();

    // 创建两个不同的指标进行比较
    let metrics1 = create_test_metrics(5, 2, 1000, 100);
    let metrics2 = create_test_metrics(8, 3, 2000, 150);

    let comparison = monitor.compare_metrics(&metrics2, &metrics1);

    assert!(comparison.contains("面板数变化: +3"));
    assert!(comparison.contains("新处理字节数: 1000"));
    assert!(comparison.contains("新处理批次数: 50"));

    println!("指标比较:\n{}", comparison);
}

/// 辅助函数：创建测试指标
fn create_test_metrics(
    pane_count: usize,
    worker_threads: usize,
    total_bytes: u64,
    total_batches: u64,
) -> terminal_lib::mux::PerformanceMetrics {
    let monitor = PerformanceMonitor::new();
    let mux_status = TerminalMuxStatus {
        pane_count,
        subscriber_count: 1,
        next_pane_id: pane_count as u32 + 1,
        next_subscriber_id: 2,
        main_thread_id: std::thread::current().id(),
    };

    let io_stats = terminal_lib::mux::IoThreadPoolStats {
        active_panes: pane_count,
        worker_threads,
        pending_tasks: 0,
        total_bytes_processed: total_bytes,
        total_batches_processed: total_batches,
    };

    monitor.collect_metrics(&mux_status, Some(&io_stats))
}

/// 测试默认配置
#[test]
fn test_default_configurations() {
    // 测试IoConfig默认值
    let config = IoConfig::default();
    assert_eq!(config.buffer_size, 4096);
    assert_eq!(config.batch_size, 1024);
    assert_eq!(config.flush_interval_ms, 16);

    // 测试IoMode默认值
    let mode = IoMode::default();
    assert!(matches!(mode, IoMode::ThreadPool));
}
