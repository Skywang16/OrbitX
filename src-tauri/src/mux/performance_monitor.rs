//! 性能监控模块
//!
//! 监控内存使用、线程数量和I/O性能指标

use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, warn};

use crate::mux::{IoThreadPoolStats, TerminalMuxStatus};

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 当前活跃面板数量
    pub active_panes: usize,
    /// 工作线程数量
    pub worker_threads: usize,
    /// 总处理字节数
    pub total_bytes_processed: u64,
    /// 总处理批次数
    pub total_batches_processed: u64,
    /// 平均批次大小
    pub avg_batch_size: f64,
    /// 内存使用估算（字节）
    pub estimated_memory_usage: u64,
    /// 监控开始时间
    pub monitoring_start_time: Instant,
    /// 运行时长（秒）
    pub uptime_seconds: u64,
}

/// 性能监控器
pub struct PerformanceMonitor {
    start_time: Instant,
    last_metrics: Arc<std::sync::Mutex<Option<PerformanceMetrics>>>,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        debug!("启动性能监控器");
        Self {
            start_time: Instant::now(),
            last_metrics: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// 收集性能指标
    pub fn collect_metrics(
        &self,
        mux_status: &TerminalMuxStatus,
        io_stats: Option<&IoThreadPoolStats>,
    ) -> PerformanceMetrics {
        let uptime = self.start_time.elapsed();

        let (worker_threads, total_bytes, total_batches, avg_batch_size, estimated_memory) =
            if let Some(stats) = io_stats {
                let avg_batch = if stats.total_batches_processed > 0 {
                    stats.total_bytes_processed as f64 / stats.total_batches_processed as f64
                } else {
                    0.0
                };

                // 估算内存使用：
                // - 每个活跃面板约 64KB 缓冲区
                // - 每个工作线程约 32KB 栈空间
                // - 通道缓冲区约 1MB
                let memory_estimate = (stats.active_panes * 64 * 1024)
                    + (stats.worker_threads * 32 * 1024)
                    + (1024 * 1024);

                (
                    stats.worker_threads,
                    stats.total_bytes_processed,
                    stats.total_batches_processed,
                    avg_batch,
                    memory_estimate as u64,
                )
            } else {
                // 传统模式估算：每个面板2个线程，每个线程约32KB
                let estimated_threads = mux_status.pane_count * 2;
                let memory_estimate = estimated_threads * 32 * 1024;

                (estimated_threads, 0, 0, 0.0, memory_estimate as u64)
            };

        let metrics = PerformanceMetrics {
            active_panes: mux_status.pane_count,
            worker_threads,
            total_bytes_processed: total_bytes,
            total_batches_processed: total_batches,
            avg_batch_size,
            estimated_memory_usage: estimated_memory,
            monitoring_start_time: self.start_time,
            uptime_seconds: uptime.as_secs(),
        };

        // 更新缓存的指标
        if let Ok(mut last_metrics) = self.last_metrics.lock() {
            *last_metrics = Some(metrics.clone());
        }

        debug!("收集性能指标: {:?}", metrics);
        metrics
    }

    /// 获取上次收集的指标
    pub fn get_last_metrics(&self) -> Option<PerformanceMetrics> {
        self.last_metrics
            .lock()
            .ok()
            .and_then(|metrics| metrics.clone())
    }

    /// 检查性能警告
    pub fn check_performance_warnings(&self, metrics: &PerformanceMetrics) {
        // 检查内存使用
        if metrics.estimated_memory_usage > 100 * 1024 * 1024 {
            // 超过100MB
            warn!(
                "内存使用较高: {:.2} MB，活跃面板数: {}",
                metrics.estimated_memory_usage as f64 / (1024.0 * 1024.0),
                metrics.active_panes
            );
        }

        // 检查面板数量
        if metrics.active_panes > 50 {
            warn!("活跃面板数量较多: {}", metrics.active_panes);
        }

        // 检查批次大小效率
        if metrics.total_batches_processed > 0 && metrics.avg_batch_size < 100.0 {
            warn!(
                "平均批次大小较小: {:.2} 字节，可能影响性能",
                metrics.avg_batch_size
            );
        }
    }

    /// 生成性能报告
    pub fn generate_report(&self, metrics: &PerformanceMetrics) -> String {
        let memory_mb = metrics.estimated_memory_usage as f64 / (1024.0 * 1024.0);
        let throughput = if metrics.uptime_seconds > 0 {
            metrics.total_bytes_processed as f64 / metrics.uptime_seconds as f64
        } else {
            0.0
        };

        format!(
            r#"
=== OrbitX 性能报告 ===
运行时长: {} 秒
活跃面板: {}
工作线程: {}
估算内存使用: {:.2} MB
总处理字节数: {} 字节
总处理批次数: {}
平均批次大小: {:.2} 字节
平均吞吐量: {:.2} 字节/秒
========================
"#,
            metrics.uptime_seconds,
            metrics.active_panes,
            metrics.worker_threads,
            memory_mb,
            metrics.total_bytes_processed,
            metrics.total_batches_processed,
            metrics.avg_batch_size,
            throughput
        )
    }

    /// 比较两个指标的差异
    pub fn compare_metrics(
        &self,
        current: &PerformanceMetrics,
        previous: &PerformanceMetrics,
    ) -> String {
        let memory_diff =
            current.estimated_memory_usage as i64 - previous.estimated_memory_usage as i64;
        let pane_diff = current.active_panes as i64 - previous.active_panes as i64;
        let bytes_diff = current.total_bytes_processed - previous.total_bytes_processed;
        let batch_diff = current.total_batches_processed - previous.total_batches_processed;

        format!(
            r#"
=== 性能指标变化 ===
面板数变化: {:+}
内存使用变化: {:+} KB
新处理字节数: {} 字节
新处理批次数: {}
==================
"#,
            pane_diff,
            memory_diff / 1024,
            bytes_diff,
            batch_diff
        )
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能监控工具函数
pub mod utils {

    /// 获取系统线程数量（近似值）
    pub fn get_system_thread_count() -> Option<usize> {
        // 这是一个简化的实现，实际应用中可能需要使用系统API
        // 在Unix系统上可以读取/proc/self/status或使用系统调用
        std::thread::available_parallelism().map(|n| n.get()).ok()
    }

    /// 获取进程内存使用（近似值）
    pub fn get_process_memory_usage() -> Option<u64> {
        // 这是一个占位符实现
        // 实际应用中需要使用平台特定的API来获取真实的内存使用情况
        None
    }

    /// 格式化字节数为人类可读格式
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// 格式化持续时间为人类可读格式
    pub fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.get_last_metrics().is_none());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(utils::format_bytes(1024), "1.00 KB");
        assert_eq!(utils::format_bytes(1048576), "1.00 MB");
        assert_eq!(utils::format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(utils::format_duration(30), "30s");
        assert_eq!(utils::format_duration(90), "1m 30s");
        assert_eq!(utils::format_duration(3661), "1h 1m 1s");
    }
}
