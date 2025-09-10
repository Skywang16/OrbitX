/*!
 * 性能监控和优化机制
 *
 * 提供向量索引系统的性能监控、统计和优化建议
 * 基于实际运行数据进行性能调优和预警
 */

use crate::vector_index::constants::performance::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// 性能指标类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// 文件解析时间
    FileParsing,
    /// 向量化时间
    Vectorization,
    /// 向量上传时间
    VectorUpload,
    /// 搜索响应时间
    SearchResponse,
    /// 批处理时间
    BatchProcessing,
    /// 内存使用量
    MemoryUsage,
    /// 错误率
    ErrorRate,
}

/// 性能指标记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: u64,
    pub context: HashMap<String, String>,
}

/// 性能统计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceStats {
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 95%百分位响应时间
    pub p95_response_time: f64,
    /// 吞吐量（操作/秒）
    pub throughput: f64,
    /// 错误率（0-1）
    pub error_rate: f64,
    /// 内存使用峰值（MB）
    pub peak_memory_mb: f64,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 性能监控器
pub struct PerformanceMonitor {
    /// 指标历史记录（最近1000条）
    metrics_history: Arc<Mutex<VecDeque<PerformanceMetric>>>,
    /// 当前活跃的操作追踪
    active_operations: Arc<Mutex<HashMap<String, Instant>>>,
    /// 配置参数
    config: MonitorConfig,
}

/// 监控配置
#[derive(Debug, Clone)]
struct MonitorConfig {
    /// 历史记录保留数量
    max_history_size: usize,
    /// 性能警告阈值
    warning_thresholds: HashMap<MetricType, f64>,
    /// 是否启用详细监控
    #[allow(dead_code)]
    detailed_monitoring: bool,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        let mut warning_thresholds = HashMap::new();
        warning_thresholds.insert(MetricType::FileParsing, FILE_PARSING_THRESHOLD);
        warning_thresholds.insert(MetricType::Vectorization, VECTORIZATION_THRESHOLD);
        warning_thresholds.insert(MetricType::VectorUpload, VECTOR_UPLOAD_THRESHOLD);
        warning_thresholds.insert(MetricType::SearchResponse, SEARCH_RESPONSE_THRESHOLD);
        warning_thresholds.insert(MetricType::ErrorRate, ERROR_RATE_THRESHOLD);

        Self {
            metrics_history: Arc::new(Mutex::new(VecDeque::new())),
            active_operations: Arc::new(Mutex::new(HashMap::new())),
            config: MonitorConfig {
                max_history_size: MAX_HISTORY_SIZE,
                warning_thresholds,
                detailed_monitoring: true,
            },
        }
    }

    /// 开始追踪操作
    pub fn start_operation(&self, operation_id: String) {
        if let Ok(mut operations) = self.active_operations.lock() {
            operations.insert(operation_id, Instant::now());
        }
    }

    /// 结束操作并记录指标
    pub fn end_operation(&self, operation_id: String, metric_type: MetricType) -> Option<Duration> {
        let start_time = {
            if let Ok(mut operations) = self.active_operations.lock() {
                operations.remove(&operation_id)
            } else {
                return None;
            }
        };

        if let Some(start) = start_time {
            let duration = start.elapsed();
            self.record_metric(metric_type, duration.as_millis() as f64, HashMap::new());
            Some(duration)
        } else {
            warn!("未找到操作记录: {}", operation_id);
            None
        }
    }

    /// 记录性能指标
    pub fn record_metric(
        &self,
        metric_type: MetricType,
        value: f64,
        context: HashMap<String, String>,
    ) {
        let metric = PerformanceMetric {
            metric_type: metric_type.clone(),
            value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            context,
        };

        // 检查是否超过警告阈值
        if let Some(threshold) = self.config.warning_thresholds.get(&metric_type) {
            if value > *threshold {
                warn!(
                    "性能警告: {:?} 超过阈值 {:.2} (当前值: {:.2})",
                    metric_type, threshold, value
                );
            }
        }

        // 添加到历史记录
        if let Ok(mut history) = self.metrics_history.lock() {
            history.push_back(metric);

            // 保持历史记录大小限制
            while history.len() > self.config.max_history_size {
                history.pop_front();
            }
        }

        debug!("记录性能指标: {:?} = {:.2}", metric_type, value);
    }

    /// 获取性能统计摘要
    pub fn get_performance_stats(&self) -> PerformanceStats {
        let metrics = if let Ok(history) = self.metrics_history.lock() {
            history.clone()
        } else {
            VecDeque::new()
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 计算响应时间统计（最近5分钟）
        let five_minutes_ago = now.saturating_sub(FIVE_MINUTES_MS);
        let recent_response_times: Vec<f64> = metrics
            .iter()
            .filter(|m| {
                m.timestamp >= five_minutes_ago
                    && matches!(
                        m.metric_type,
                        MetricType::SearchResponse
                            | MetricType::FileParsing
                            | MetricType::Vectorization
                    )
            })
            .map(|m| m.value)
            .collect();

        let avg_response_time = if recent_response_times.is_empty() {
            0.0
        } else {
            recent_response_times.iter().sum::<f64>() / recent_response_times.len() as f64
        };

        let p95_response_time = self.calculate_percentile(&recent_response_times, 0.95);

        // 计算吞吐量（操作/秒）
        let recent_operations_count = recent_response_times.len() as f64;
        let throughput = recent_operations_count / 300.0; // 5分钟 = 300秒

        // 计算错误率
        let error_metrics: Vec<&PerformanceMetric> = metrics
            .iter()
            .filter(|m| {
                m.timestamp >= five_minutes_ago && matches!(m.metric_type, MetricType::ErrorRate)
            })
            .collect();

        let error_rate = if error_metrics.is_empty() {
            0.0
        } else {
            error_metrics.iter().map(|m| m.value).sum::<f64>() / error_metrics.len() as f64
        };

        // 计算内存使用峰值
        let memory_metrics: Vec<f64> = metrics
            .iter()
            .filter(|m| {
                m.timestamp >= five_minutes_ago && matches!(m.metric_type, MetricType::MemoryUsage)
            })
            .map(|m| m.value)
            .collect();

        let peak_memory_mb = memory_metrics
            .iter()
            .fold(0.0_f64, |max, &val| max.max(val));

        PerformanceStats {
            avg_response_time,
            p95_response_time,
            throughput,
            error_rate,
            peak_memory_mb,
            last_updated: now,
        }
    }

    /// 计算百分位数
    fn calculate_percentile(&self, values: &[f64], percentile: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = ((sorted_values.len() as f64 - 1.0) * percentile).round() as usize;
        sorted_values.get(index).copied().unwrap_or(0.0)
    }

    /// 生成性能优化建议
    pub fn generate_optimization_suggestions(&self) -> Vec<String> {
        let stats = self.get_performance_stats();
        let mut suggestions = Vec::new();

        // 响应时间建议
        if stats.avg_response_time > 1000.0 {
            suggestions.push("平均响应时间过长，建议检查网络连接或增加并发处理数量".to_string());
        }

        if stats.p95_response_time > 5000.0 {
            suggestions.push("95%响应时间过长，可能存在性能瓶颈".to_string());
        }

        // 吞吐量建议
        if stats.throughput < 0.5 {
            suggestions.push("处理吞吐量较低，建议优化批处理大小或增加并发度".to_string());
        }

        // 错误率建议
        if stats.error_rate > 0.05 {
            suggestions.push(format!(
                "错误率较高 ({:.1}%)，建议检查配置和网络连接",
                stats.error_rate * 100.0
            ));
        }

        // 内存使用建议
        if stats.peak_memory_mb > 1000.0 {
            suggestions.push("内存使用峰值较高，建议调整批处理大小以降低内存压力".to_string());
        }

        suggestions
    }

    /// 检查性能健康状况
    pub fn check_health(&self) -> HealthStatus {
        let stats = self.get_performance_stats();
        let mut issues = Vec::new();

        // 检查各项指标
        if stats.avg_response_time > 2000.0 {
            issues.push("平均响应时间过长".to_string());
        }

        if stats.error_rate > 0.1 {
            issues.push("错误率过高".to_string());
        }

        if stats.throughput < 0.1 {
            issues.push("处理吞吐量过低".to_string());
        }

        let status = if issues.is_empty() {
            if stats.avg_response_time < 500.0 && stats.error_rate < 0.01 && stats.throughput > 1.0
            {
                HealthLevel::Excellent
            } else {
                HealthLevel::Good
            }
        } else if issues.len() <= 1 {
            HealthLevel::Warning
        } else {
            HealthLevel::Critical
        };

        HealthStatus {
            level: status,
            issues,
            stats,
        }
    }

    /// 记录批处理性能
    pub fn record_batch_performance(
        &self,
        batch_size: usize,
        duration: Duration,
        success_count: usize,
    ) {
        let mut context = HashMap::new();
        context.insert("batch_size".to_string(), batch_size.to_string());
        context.insert("success_count".to_string(), success_count.to_string());
        context.insert(
            "success_rate".to_string(),
            format!("{:.2}", success_count as f64 / batch_size as f64),
        );

        self.record_metric(
            MetricType::BatchProcessing,
            duration.as_millis() as f64,
            context,
        );

        // 记录错误率
        let error_rate = 1.0 - (success_count as f64 / batch_size as f64);
        self.record_metric(MetricType::ErrorRate, error_rate, HashMap::new());
    }

    /// 记录内存使用情况
    pub fn record_memory_usage(&self) {
        if let Ok(memory_info) = sys_info::mem_info() {
            let used_mb = (memory_info.total - memory_info.free) as f64 / 1024.0;
            self.record_metric(MetricType::MemoryUsage, used_mb, HashMap::new());
        }
    }

    /// 获取最近的指标历史
    pub fn get_recent_metrics(
        &self,
        metric_type: MetricType,
        count: usize,
    ) -> Vec<PerformanceMetric> {
        if let Ok(history) = self.metrics_history.lock() {
            history
                .iter()
                .filter(|m| {
                    std::mem::discriminant(&m.metric_type) == std::mem::discriminant(&metric_type)
                })
                .rev()
                .take(count)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// 健康状况级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthLevel {
    Excellent,
    Good,
    Warning,
    Critical,
}

/// 健康状况报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub level: HealthLevel,
    pub issues: Vec<String>,
    pub stats: PerformanceStats,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// 真实的系统内存监控
mod sys_info {
    pub struct MemInfo {
        pub total: u64,
        pub free: u64,
    }

    pub fn mem_info() -> Result<MemInfo, &'static str> {
        // 使用标准库获取真实内存信息
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // 使用vm_stat命令获取内存信息
            if let Ok(output) = Command::new("vm_stat").output() {
                let output_str = String::from_utf8_lossy(&output.stdout);

                // 解析页面大小和统计信息
                let mut free_pages = 0u64;
                let mut total_pages = 0u64;

                for line in output_str.lines() {
                    if line.contains("Pages free:") {
                        if let Some(num_str) = line.split_whitespace().nth(2) {
                            free_pages = num_str.trim_end_matches('.').parse().unwrap_or(0);
                        }
                    } else if line.contains("Pages active:")
                        || line.contains("Pages inactive:")
                        || line.contains("Pages speculative:")
                        || line.contains("Pages wired down:")
                    {
                        if let Some(num_str) = line.split_whitespace().last() {
                            let pages: u64 = num_str.trim_end_matches('.').parse().unwrap_or(0);
                            total_pages += pages;
                        }
                    }
                }

                total_pages += free_pages;
                // 从vm_stat输出中提取页面大小，通常是16KB
                let page_size = if output_str.contains("page size of") {
                    // 尝试从输出中解析页面大小
                    if let Some(size_line) = output_str
                        .lines()
                        .find(|line| line.contains("page size of"))
                    {
                        if let Some(size_str) = size_line.split("page size of ").nth(1) {
                            if let Some(size_part) = size_str.split(" bytes").next() {
                                size_part.parse().unwrap_or(16384u64)
                            } else {
                                16384u64
                            }
                        } else {
                            16384u64
                        }
                    } else {
                        16384u64
                    }
                } else {
                    16384u64 // macOS默认页面大小
                };

                return Ok(MemInfo {
                    total: total_pages * page_size / 1024, // KB
                    free: free_pages * page_size / 1024,   // KB
                });
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs;

            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                let mut total_kb = 0u64;
                let mut available_kb = 0u64;

                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            total_kb = kb_str.parse().unwrap_or(0);
                        }
                    } else if line.starts_with("MemAvailable:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            available_kb = kb_str.parse().unwrap_or(0);
                        }
                    }
                }

                return Ok(MemInfo {
                    total: total_kb,
                    free: available_kb,
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Windows可以使用GlobalMemoryStatus API，这里提供备用实现
            // 实际项目中建议使用winapi crate
        }

        // 备用方案：返回合理的默认值
        Ok(MemInfo {
            total: 8 * 1024 * 1024, // 8GB
            free: 4 * 1024 * 1024,  // 4GB (estimated)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();

        // 测试指标记录
        monitor.record_metric(MetricType::SearchResponse, 150.0, HashMap::new());
        monitor.record_metric(MetricType::SearchResponse, 200.0, HashMap::new());

        let stats = monitor.get_performance_stats();
        assert_eq!(stats.avg_response_time, 175.0);
    }

    #[test]
    fn test_operation_tracking() {
        let monitor = PerformanceMonitor::new();

        monitor.start_operation("test_op".to_string());
        std::thread::sleep(Duration::from_millis(10));
        let duration = monitor.end_operation("test_op".to_string(), MetricType::FileParsing);

        assert!(duration.is_some());
        assert!(duration.unwrap().as_millis() >= 10);
    }

    #[test]
    fn test_percentile_calculation() {
        let monitor = PerformanceMonitor::new();
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(monitor.calculate_percentile(&values, 0.5), 3.0);
        assert_eq!(monitor.calculate_percentile(&values, 0.95), 5.0);
    }
}
