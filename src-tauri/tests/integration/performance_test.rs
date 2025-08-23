/*!
 * 性能测试模块
 *
 * 测试多终端并发操作的稳定性和性能
 */

use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

use std::sync::Arc;
use terminal_lib::mux::{get_mux, PtySize, TerminalMux};

/// 基本性能测试配置
#[derive(Debug)]
struct PerformanceTestConfig {
    /// 并发终端数量
    concurrent_terminals: usize,
    /// 每个终端的写入次数
    writes_per_terminal: usize,
    /// 写入数据大小（字节）
    write_data_size: usize,

}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            concurrent_terminals: 5,
            writes_per_terminal: 100,
            write_data_size: 1024,

        }
    }
}

/// 性能测试结果
#[derive(Debug)]
pub(crate) struct PerformanceTestResult {
    /// 成功创建的终端数量
    terminals_created: usize,
    /// 成功的写入操作数量
    successful_writes: usize,
    /// 失败的写入操作数量
    failed_writes: usize,

    /// 平均每秒写入次数
    writes_per_second: f64,

    /// 内存使用（MB），可能在部分平台不可用
    memory_usage_mb: Option<f64>,
}

/// 执行基本性能测试
pub(crate) async fn run_basic_performance_test(
) -> Result<PerformanceTestResult, Box<dyn std::error::Error>> {
    let config = PerformanceTestConfig::default();
    info!("开始基本性能测试，配置: {:?}", config);

    let start_time = Instant::now();
    let mux = get_mux();

    // 记录初始状态
    let initial_pane_count = mux.pane_count();
    info!("测试开始时面板数量: {}", initial_pane_count);

    // 创建多个终端
    let mut pane_ids = Vec::new();
    let mut terminals_created = 0;

    for i in 0..config.concurrent_terminals {
        match mux.create_pane(PtySize::new(24, 80)).await {
            Ok(pane_id) => {
                pane_ids.push(pane_id);
                terminals_created += 1;
                info!("创建终端 {}: {:?}", i + 1, pane_id);
            }
            Err(e) => {
                warn!("创建终端 {} 失败: {}", i + 1, e);
            }
        }

        // 短暂延迟避免过快创建
        sleep(Duration::from_millis(10)).await;
    }

    info!("成功创建 {} 个终端", terminals_created);

    // 准备测试数据
    let test_data = "a".repeat(config.write_data_size);
    let test_data_bytes = test_data.as_bytes();

    // 并发写入测试
    let mut successful_writes = 0;
    let mut failed_writes = 0;

    let write_start_time = Instant::now();

    // 使用 tokio 的并发任务
    let mut tasks = Vec::new();

    for pane_id in &pane_ids {
        let pane_id = *pane_id;
        let data = test_data_bytes.to_vec();
        let writes_count = config.writes_per_terminal;

        let task = tokio::spawn(async move {
            let mut local_successful = 0;
            let mut local_failed = 0;

            for i in 0..writes_count {
                match get_mux().write_to_pane(pane_id, &data) {
                    Ok(_) => {
                        local_successful += 1;
                    }
                    Err(e) => {
                        warn!("写入失败 (面板 {:?}, 第 {} 次): {}", pane_id, i + 1, e);
                        local_failed += 1;
                    }
                }

                // 短暂延迟模拟真实使用场景
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            (local_successful, local_failed)
        });

        tasks.push(task);
    }

    // 等待所有写入任务完成
    for task in tasks {
        match task.await {
            Ok((s, f)) => {
                successful_writes += s;
                failed_writes += f;
            }
            Err(e) => {
                warn!("写入任务失败: {}", e);
                failed_writes += config.writes_per_terminal;
            }
        }
    }

    let write_duration = write_start_time.elapsed();
    info!("写入测试完成，耗时: {:?}", write_duration);

    // 等待一段时间让 I/O 处理完成
    info!("等待 I/O 处理完成...");
    sleep(Duration::from_secs(2)).await;

    // 清理终端
    let mut cleanup_successful = 0;
    for pane_id in pane_ids {
        match mux.remove_pane(pane_id) {
            Ok(_) => {
                cleanup_successful += 1;
                info!("清理终端成功: {:?}", pane_id);
            }
            Err(e) => {
                warn!("清理终端失败 {:?}: {}", pane_id, e);
            }
        }
    }

    info!("清理了 {} 个终端", cleanup_successful);

    // 验证清理效果
    let final_pane_count = mux.pane_count();
    if final_pane_count != initial_pane_count {
        warn!(
            "面板数量不匹配！初始: {}, 最终: {}",
            initial_pane_count, final_pane_count
        );
    } else {
        info!("面板清理验证通过");
    }

    let total_duration = start_time.elapsed();
    let writes_per_second = if write_duration.as_secs_f64() > 0.0 {
        successful_writes as f64 / write_duration.as_secs_f64()
    } else {
        0.0
    };

    let result = PerformanceTestResult {
        terminals_created,
        successful_writes,
        failed_writes,

        writes_per_second,
        memory_usage_mb: get_memory_usage(),
    };

    info!("性能测试完成: {:?}", result);
    Ok(result)
}

/// 执行并发稳定性测试
pub async fn run_concurrent_stability_test() -> Result<(), Box<dyn std::error::Error>> {
    info!("开始并发稳定性测试");

    let mux = get_mux();
    let test_duration = Duration::from_secs(10);
    let start_time = Instant::now();

    // 创建多个并发任务
    let mut tasks = Vec::new();

    // 任务1: 持续创建和删除终端
    let task1 = tokio::spawn(async move {
        let mut created_count = 0;
        let mut deleted_count = 0;

        while start_time.elapsed() < test_duration {
            // 创建终端
            match get_mux().create_pane(PtySize::new(24, 80)).await {
                Ok(pane_id) => {
                    created_count += 1;

                    // 短暂使用后删除
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    match get_mux().remove_pane(pane_id) {
                        Ok(_) => deleted_count += 1,
                        Err(e) => warn!("删除终端失败: {}", e),
                    }
                }
                Err(e) => warn!("创建终端失败: {}", e),
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        info!(
            "任务1完成: 创建 {} 个，删除 {} 个终端",
            created_count, deleted_count
        );
    });

    // 任务2: 对现有终端进行写入操作
    let task2 = tokio::spawn(async move {
        let mut write_count = 0;

        while start_time.elapsed() < test_duration {
            let panes = get_mux().list_panes();

            for pane_id in panes {
                match get_mux().write_to_pane(pane_id, b"test data") {
                    Ok(_) => write_count += 1,
                    Err(_) => {} // 忽略写入失败，可能是终端已被删除
                }
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        info!("任务2完成: 执行了 {} 次写入操作", write_count);
    });

    // 任务3: 监控系统状态
    let task3 = tokio::spawn(async move {
        let mut max_panes = 0;

        while start_time.elapsed() < test_duration {
            let current_panes = get_mux().pane_count();
            if current_panes > max_panes {
                max_panes = current_panes;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("任务3完成: 最大面板数量 {}", max_panes);
    });

    tasks.push(task1);
    tasks.push(task2);
    tasks.push(task3);

    // 等待所有任务完成
    for task in tasks {
        if let Err(e) = task.await {
            warn!("并发任务失败: {}", e);
        }
    }

    // 最终清理
    let remaining_panes = mux.list_panes();
    for pane_id in remaining_panes {
        let _ = mux.remove_pane(pane_id);
    }

    info!("并发稳定性测试完成");
    Ok(())
}

/// 获取内存使用情况（简化版本）
fn get_memory_usage() -> Option<f64> {
    // 在实际应用中，可以使用 sysinfo 或其他库获取内存使用情况
    // 这里返回 None 表示暂不支持
    None
}

/// 运行所有性能测试
pub async fn run_all_performance_tests() -> Result<(), Box<dyn std::error::Error>> {
    info!("开始运行所有性能测试");

    // 基本性能测试
    match run_basic_performance_test().await {
        Ok(result) => {
            info!("基本性能测试通过: {:?}", result);

            // 检查关键指标
            if result.failed_writes > result.successful_writes / 10 {
                warn!(
                    "写入失败率过高: {}/{}",
                    result.failed_writes, result.successful_writes
                );
            }

            if result.writes_per_second < 100.0 {
                warn!("写入性能较低: {:.2} writes/sec", result.writes_per_second);
            } else {
                info!("写入性能良好: {:.2} writes/sec", result.writes_per_second);
            }
        }
        Err(e) => {
            warn!("基本性能测试失败: {}", e);
        }
    }

    // 并发稳定性测试
    match run_concurrent_stability_test().await {
        Ok(_) => info!("并发稳定性测试通过"),
        Err(e) => warn!("并发稳定性测试失败: {}", e),
    }

    info!("所有性能测试完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_performance() {
        // 初始化日志
        let _ = tracing_subscriber::fmt::try_init();

        let result = run_basic_performance_test().await;
        assert!(result.is_ok(), "基本性能测试应该成功");

        let result = result.unwrap();
        assert!(result.terminals_created > 0, "应该创建至少一个终端");
        assert!(result.successful_writes > 0, "应该有成功的写入操作");
    }

    #[tokio::test]
    async fn test_concurrent_stability() {
        // 初始化日志
        let _ = tracing_subscriber::fmt::try_init();

        let result = run_concurrent_stability_test().await;
        assert!(result.is_ok(), "并发稳定性测试应该成功");
    }

    #[tokio::test]
    async fn test_memory_cleanup() {
        // 初始化日志
        let _ = tracing_subscriber::fmt::try_init();

        // 创建独立的Mux实例，避免与其他测试冲突
        let mux = Arc::new(TerminalMux::new());

        let initial_count = mux.pane_count();
        info!("测试开始时面板数量: {}", initial_count);

        // 创建一些终端
        let mut pane_ids = Vec::new();
        for i in 0..3 {
            let pane_id = mux.create_pane(PtySize::default()).await.unwrap();
            pane_ids.push(pane_id);
            info!("创建面板 {}: {:?}", i + 1, pane_id);
        }

        // 验证面板数量增加了3个
        let current_count = mux.pane_count();
        info!("创建后面板数量: {}", current_count);
        assert_eq!(
            current_count,
            initial_count + 3,
            "面板数量不匹配: 期望 {}, 实际 {}",
            initial_count + 3,
            current_count
        );

        // 清理终端
        for (i, pane_id) in pane_ids.into_iter().enumerate() {
            match mux.remove_pane(pane_id) {
                Ok(_) => info!("清理面板 {}: {:?}", i + 1, pane_id),
                Err(e) => warn!("清理面板 {} 失败: {}", i + 1, e),
            }
        }

        // 等待I/O线程完全关闭，使用重试机制
        let mut retry_count = 0;
        let max_retries = 20; // 最多重试20次，总共2秒

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let current_count = mux.pane_count();

            if current_count == initial_count {
                info!("面板清理完成，重试次数: {}", retry_count);
                break;
            }

            retry_count += 1;
            if retry_count >= max_retries {
                warn!(
                    "面板清理超时，当前数量: {}, 期望: {}",
                    current_count, initial_count
                );
                break;
            }

            info!(
                "等待面板清理完成，当前数量: {}, 期望: {}, 重试: {}/{}",
                current_count, initial_count, retry_count, max_retries
            );
        }

        // 验证面板数量回到初始状态
        let final_count = mux.pane_count();
        info!("最终面板数量: {}", final_count);

        // 如果数量不匹配，显示详细信息
        if final_count != initial_count {
            let remaining_panes = mux.list_panes();
            warn!("剩余面板: {:?}", remaining_panes);
        }

        assert_eq!(
            final_count, initial_count,
            "面板清理后数量不匹配: 期望 {initial_count}, 实际 {final_count}"
        );
    }
}
