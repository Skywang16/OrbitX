use std::time::Instant;
use tracing::warn;

use crate::mux::TerminalMuxStatus;

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub active_panes: usize,
    pub reader_threads: usize,
    pub estimated_memory_usage: u64,
    pub monitoring_start_time: Instant,
    pub uptime_seconds: u64,
}

pub struct PerformanceMonitor {
    start_time: Instant,
    last_metrics: std::sync::Mutex<Option<PerformanceMetrics>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_metrics: std::sync::Mutex::new(None),
        }
    }

    pub fn collect_metrics(&self, mux_status: &TerminalMuxStatus) -> PerformanceMetrics {
        let uptime = self.start_time.elapsed();
        let reader_threads = mux_status.pane_count;
        let estimated_memory_usage = (reader_threads * 64 * 1024) as u64;

        let metrics = PerformanceMetrics {
            active_panes: mux_status.pane_count,
            reader_threads,
            estimated_memory_usage,
            monitoring_start_time: self.start_time,
            uptime_seconds: uptime.as_secs(),
        };

        if let Ok(mut last_metrics) = self.last_metrics.lock() {
            *last_metrics = Some(metrics.clone());
        }

        metrics
    }

    pub fn get_last_metrics(&self) -> Option<PerformanceMetrics> {
        self.last_metrics
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    pub fn check_performance_warnings(&self, metrics: &PerformanceMetrics) {
        if metrics.estimated_memory_usage > 100 * 1024 * 1024 {
            warn!(
                "内存使用较高: {:.2} MB，活跃面板数: {}",
                metrics.estimated_memory_usage as f64 / (1024.0 * 1024.0),
                metrics.active_panes
            );
        }

        if metrics.active_panes > 50 {
            warn!("活跃面板数量较多: {}", metrics.active_panes);
        }
    }

    pub fn generate_report(&self, metrics: &PerformanceMetrics) -> String {
        let memory_mb = metrics.estimated_memory_usage as f64 / (1024.0 * 1024.0);

        format!(
            r#"
=== OrbitX 性能报告 ===
运行时长: {} 秒
活跃面板: {}
读线程: {}
估算内存使用: {:.2} MB
========================
"#,
            metrics.uptime_seconds, metrics.active_panes, metrics.reader_threads, memory_mb
        )
    }

    pub fn compare_metrics(
        &self,
        current: &PerformanceMetrics,
        previous: &PerformanceMetrics,
    ) -> String {
        let memory_diff =
            current.estimated_memory_usage as i64 - previous.estimated_memory_usage as i64;
        let pane_diff = current.active_panes as i64 - previous.active_panes as i64;

        format!(
            r#"
=== 性能指标变化 ===
面板数变化: {:+}
内存使用变化: {:+} KB
==================
"#,
            pane_diff,
            memory_diff / 1024
        )
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
