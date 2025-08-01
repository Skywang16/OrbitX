/*!
 * AI模块测试套件
 *
 * 包含AI模块所有组件的全面测试
 */

// 测试工具和模拟组件
pub mod test_data;
pub mod test_utils;

// 核心组件测试
pub mod types_tests;

pub mod adapters_tests;
pub mod cache_manager_tests;
pub mod command_processor_tests;
pub mod commands_tests;
pub mod config_tests;
pub mod context_manager_tests;
pub mod prompt_engine_tests;

// 集成测试
pub mod integration_tests;

// 重新导出常用的测试工具
pub use test_data::*;
pub use test_utils::*;

// 测试宏
#[macro_export]
macro_rules! assert_ai_error_contains {
    ($result:expr, $expected_msg:expr) => {
        match $result {
            Err(error) => {
                let error_msg = error.to_string();
                assert!(
                    error_msg.contains($expected_msg),
                    "Error message '{}' does not contain '{}'",
                    error_msg,
                    $expected_msg
                );
            }
            Ok(val) => panic!("Expected error, got Ok: {:?}", val),
        }
    };
}

#[macro_export]
macro_rules! assert_ai_success {
    ($result:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => panic!("Expected success, got error: {:?}", err),
        }
    };
}

#[macro_export]
macro_rules! assert_response_contains {
    ($response:expr, $expected:expr) => {
        assert!(
            $response.content.contains($expected),
            "Response '{}' does not contain '{}'",
            $response.content,
            $expected
        );
    };
}

#[macro_export]
macro_rules! assert_model_config_valid {
    ($config:expr) => {
        assert!(!$config.id.is_empty(), "Model ID should not be empty");
        assert!(!$config.name.is_empty(), "Model name should not be empty");
        assert!(!$config.api_url.is_empty(), "API URL should not be empty");
        assert!(!$config.model.is_empty(), "Model should not be empty");
    };
}

#[macro_export]
macro_rules! assert_cache_stats {
    ($stats:expr, hit_count: $hit:expr, miss_count: $miss:expr) => {
        assert_eq!($stats.hit_count, $hit, "Cache hit count mismatch");
        assert_eq!($stats.miss_count, $miss, "Cache miss count mismatch");
    };
}

#[macro_export]
macro_rules! assert_context_has_history {
    ($context:expr) => {
        assert!(
            $context.command_history.is_some(),
            "Context should have command history"
        );
        assert!(
            !$context.command_history.as_ref().unwrap().is_empty(),
            "Command history should not be empty"
        );
    };
}

#[macro_export]
macro_rules! assert_prompt_contains {
    ($prompt:expr, $expected:expr) => {
        assert!(
            $prompt.contains($expected),
            "Prompt '{}' does not contain '{}'",
            $prompt,
            $expected
        );
    };
}

#[macro_export]
macro_rules! assert_adapter_supports {
    ($adapter:expr, $feature:expr) => {
        assert!(
            $adapter
                .supported_features()
                .contains(&$feature.to_string()),
            "Adapter should support feature: {}",
            $feature
        );
    };
}

/// 测试运行器辅助函数
pub mod test_runner {
    use std::future::Future;
    use std::time::Duration;
    use tokio::time::timeout;

    /// 运行带超时的异步测试
    pub async fn run_with_timeout<F, T>(
        test_future: F,
        timeout_duration: Duration,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        timeout(timeout_duration, test_future)
            .await
            .map_err(|_| "Test timed out".into())?
    }

    /// 运行多个并发测试
    pub async fn run_concurrent_tests<F, T>(
        tests: Vec<F>,
    ) -> Vec<Result<T, Box<dyn std::error::Error>>>
    where
        F: Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        futures::future::join_all(tests).await
    }

    /// 重复运行测试以检查稳定性
    pub async fn run_stress_test<F, T>(
        test_factory: impl Fn() -> F,
        iterations: usize,
    ) -> Vec<Result<T, Box<dyn std::error::Error>>>
    where
        F: Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let mut results = Vec::new();
        for _ in 0..iterations {
            results.push(test_factory().await);
        }
        results
    }
}

/// 性能测试辅助函数
pub mod performance {
    use std::time::{Duration, Instant};

    /// 测量函数执行时间
    pub async fn measure_time<F, T>(f: F) -> (T, Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f.await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// 性能基准测试
    pub struct Benchmark {
        pub name: String,
        pub iterations: usize,
        pub total_time: Duration,
        pub avg_time: Duration,
        pub min_time: Duration,
        pub max_time: Duration,
    }

    impl Benchmark {
        pub fn new(name: String) -> Self {
            Self {
                name,
                iterations: 0,
                total_time: Duration::ZERO,
                avg_time: Duration::ZERO,
                min_time: Duration::MAX,
                max_time: Duration::ZERO,
            }
        }

        pub fn add_measurement(&mut self, duration: Duration) {
            self.iterations += 1;
            self.total_time += duration;
            self.avg_time = self.total_time / self.iterations as u32;
            self.min_time = self.min_time.min(duration);
            self.max_time = self.max_time.max(duration);
        }

        pub fn report(&self) -> String {
            format!(
                "Benchmark: {}\n  Iterations: {}\n  Total: {:?}\n  Average: {:?}\n  Min: {:?}\n  Max: {:?}",
                self.name, self.iterations, self.total_time, self.avg_time, self.min_time, self.max_time
            )
        }
    }

    /// 运行性能基准测试
    pub async fn run_benchmark<F, T>(
        name: &str,
        test_fn: impl Fn() -> F,
        iterations: usize,
    ) -> Benchmark
    where
        F: std::future::Future<Output = T>,
    {
        let mut benchmark = Benchmark::new(name.to_string());

        for _ in 0..iterations {
            let (_, duration) = measure_time(test_fn()).await;
            benchmark.add_measurement(duration);
        }

        benchmark
    }
}

/// 测试断言辅助函数
pub mod assertions {
    use anyhow::Error as AnyhowError;
    use termx::ai::{AICacheStats, AIResponse};

    /// 断言AI错误包含特定消息
    pub fn assert_error_message(error: &AnyhowError, expected_message: &str) {
        let error_string = error.to_string();
        assert!(
            error_string.contains(expected_message),
            "Error message '{}' does not contain '{}'",
            error_string,
            expected_message
        );
    }

    /// 断言响应质量
    pub fn assert_response_quality(response: &AIResponse) {
        assert!(
            !response.content.is_empty(),
            "Response content should not be empty"
        );
        assert!(
            !response.model_id.is_empty(),
            "Model ID should not be empty"
        );
    }

    /// 断言缓存性能
    pub fn assert_cache_performance(stats: &AICacheStats, min_hit_rate: f64) {
        assert!(
            stats.hit_rate >= min_hit_rate,
            "Cache hit rate {} is below minimum {}",
            stats.hit_rate,
            min_hit_rate
        );
    }

    /// 断言响应时间
    pub fn assert_response_time(duration: std::time::Duration, max_duration: std::time::Duration) {
        assert!(
            duration <= max_duration,
            "Response time {:?} exceeds maximum {:?}",
            duration,
            max_duration
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_performance_measurement() {
        let (result, duration) = performance::measure_time(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        })
        .await;

        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_benchmark() {
        let benchmark = performance::run_benchmark(
            "test_benchmark",
            || async { tokio::time::sleep(Duration::from_millis(1)).await },
            5,
        )
        .await;

        assert_eq!(benchmark.iterations, 5);
        assert!(benchmark.avg_time >= Duration::from_millis(1));
        println!("{}", benchmark.report());
    }

    #[tokio::test]
    async fn test_timeout_runner() {
        let result = test_runner::run_with_timeout(
            async { Ok::<i32, Box<dyn std::error::Error>>(42) },
            Duration::from_millis(100),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_error_assertion() {
        let error = anyhow::anyhow!("AI输入验证错误: test message");
        assertions::assert_error_message(&error, "test message");
    }
}
