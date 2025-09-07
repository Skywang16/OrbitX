/*!
 * 向量索引系统测试模块
 *
 * 为代码向量索引系统提供全面的测试覆盖，包括：
 * - Qdrant集成测试
 * - 向量化服务测试  
 * - 代码解析测试
 * - 端到端集成测试
 *
 * Requirements: 确保所有核心功能满足验收标准
 */

pub mod file_monitor_tests;
pub mod integration_tests;
pub mod qdrant_tests;
pub mod search_tests;
pub mod search_unit_tests;
pub mod test_fixtures;

// 重新导出测试工具
pub use test_fixtures::*;

use crate::test_utils::*;
use anyhow::Result;
use std::time::Duration;

/// 向量索引测试配置
#[derive(Debug, Clone)]
pub struct VectorIndexTestConfig {
    pub timeout: Duration,
    pub mock_qdrant: bool,
    pub test_data_size: usize,
    pub cleanup_on_drop: bool,
}

impl Default for VectorIndexTestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            mock_qdrant: true, // 默认使用模拟Qdrant
            test_data_size: 100,
            cleanup_on_drop: true,
        }
    }
}

/// 向量索引测试环境管理器
pub struct VectorIndexTestEnvironment {
    config: VectorIndexTestConfig,
    test_data: Vec<TestCodeVector>,
    cleanup_tasks: Vec<Box<dyn FnOnce() + Send>>,
}

impl VectorIndexTestEnvironment {
    /// 创建新的测试环境
    pub fn new() -> Self {
        Self::with_config(VectorIndexTestConfig::default())
    }

    /// 使用指定配置创建测试环境
    pub fn with_config(config: VectorIndexTestConfig) -> Self {
        let test_data = generate_test_code_vectors(config.test_data_size);

        Self {
            config,
            test_data,
            cleanup_tasks: Vec::new(),
        }
    }

    /// 获取测试数据
    pub fn test_data(&self) -> &[TestCodeVector] {
        &self.test_data
    }

    /// 获取配置
    pub fn config(&self) -> &VectorIndexTestConfig {
        &self.config
    }

    /// 添加清理任务
    pub fn add_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup_tasks.push(Box::new(cleanup));
    }

    /// 手动执行清理
    pub fn cleanup(&mut self) {
        for cleanup in self.cleanup_tasks.drain(..) {
            cleanup();
        }
    }
}

impl Drop for VectorIndexTestEnvironment {
    fn drop(&mut self) {
        if self.config.cleanup_on_drop {
            self.cleanup();
        }
    }
}

/// 测试断言宏 - 向量索引特定
#[macro_export]
macro_rules! assert_vector_upload_success {
    ($result:expr, $expected_count:expr) => {
        match $result {
            Ok(stats) => {
                assert_eq!(
                    stats.uploaded_vectors, $expected_count,
                    "上传的向量数量应该匹配: 期望 {}, 实际 {}",
                    $expected_count, stats.uploaded_vectors
                );
            }
            Err(e) => panic!("向量上传应该成功，但失败了: {:?}", e),
        }
    };
}

#[macro_export]
macro_rules! assert_search_results_valid {
    ($results:expr, $min_count:expr, $max_score:expr) => {
        assert!(
            $results.len() >= $min_count,
            "搜索结果数量应该至少为 {}, 实际为 {}",
            $min_count,
            $results.len()
        );

        for result in &$results {
            assert!(
                result.score <= $max_score,
                "搜索结果分数应该不超过 {}, 实际为 {}",
                $max_score,
                result.score
            );
        }
    };
}

#[macro_export]
macro_rules! assert_qdrant_connection_success {
    ($result:expr) => {
        match $result {
            Ok(message) => {
                assert!(
                    message.contains("连接成功"),
                    "连接成功消息应该包含 '连接成功', 实际消息: {}",
                    message
                );
            }
            Err(e) => panic!("Qdrant连接应该成功，但失败了: {:?}", e),
        }
    };
}

/// 性能测试辅助函数
pub async fn benchmark_vector_upload(
    vectors: Vec<terminal_lib::vector_index::types::CodeVector>,
    batch_size: usize,
) -> Result<(Duration, f64)> {
    let start = std::time::Instant::now();

    // 这里应该调用实际的上传函数
    // 暂时模拟
    tokio::time::sleep(Duration::from_millis(10)).await;

    let duration = start.elapsed();
    let throughput = vectors.len() as f64 / duration.as_secs_f64();

    Ok((duration, throughput))
}

/// 内存测试辅助函数
pub fn measure_memory_usage<F, R>(test_fn: F) -> (R, usize)
where
    F: FnOnce() -> R,
{
    // 简化的内存测量 - 实际应用中可能需要更精确的测量
    let result = test_fn();
    let memory_usage = std::mem::size_of_val(&result);

    (result, memory_usage)
}

/// 并发测试辅助函数
pub async fn run_concurrent_vector_operations<F, Fut>(
    operation: F,
    concurrency: usize,
) -> Result<Vec<Result<()>>>
where
    F: Fn(usize) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send,
{
    let mut handles = Vec::new();

    for i in 0..concurrency {
        let op = operation.clone();
        let handle = tokio::spawn(async move { op(i).await });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    Ok(results)
}
