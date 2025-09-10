/*!
 * 统一测试工具库
 *
 * 提供跨所有测试模块的通用工具、宏和辅助函数
 * 减少重复代码，提高测试的维护性
 */

pub mod assertions;
pub mod builders;

// 重新导出常用的工具
pub use assertions::*;
pub use builders::*;

/// 测试结果类型别名
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 通用测试配置
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout: std::time::Duration,
    pub retry_count: usize,
    pub cleanup_on_drop: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
            retry_count: 3,
            cleanup_on_drop: true,
        }
    }
}

/// 测试环境管理器
pub struct TestEnvironment {
    config: TestConfig,
    cleanup_tasks: Vec<Box<dyn FnOnce() + Send>>,
}

impl TestEnvironment {
    /// 创建新的测试环境
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    /// 使用指定配置创建测试环境
    pub fn with_config(config: TestConfig) -> Self {
        Self {
            config,
            cleanup_tasks: Vec::new(),
        }
    }

    /// 添加清理任务
    pub fn add_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup_tasks.push(Box::new(cleanup));
    }

    /// 获取配置
    pub fn config(&self) -> &TestConfig {
        &self.config
    }

    /// 手动执行清理
    pub fn cleanup(&mut self) {
        for cleanup in self.cleanup_tasks.drain(..) {
            cleanup();
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if self.config.cleanup_on_drop {
            self.cleanup();
        }
    }
}

/// 测试重试宏
#[macro_export]
macro_rules! retry_test {
    ($test_fn:expr, $max_retries:expr) => {{
        let mut last_error = None;
        for attempt in 1..=$max_retries {
            match $test_fn() {
                Ok(result) => break Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < $max_retries {
                        eprintln!("测试失败，第 {} 次重试...", attempt);
                        std::thread::sleep(std::time::Duration::from_millis(100 * attempt));
                    }
                }
            }
        }
        last_error.map(Err).unwrap_or(Ok(()))
    }};
}

/// 异步测试重试宏
#[macro_export]
macro_rules! retry_async_test {
    ($test_fn:expr, $max_retries:expr) => {{
        let mut last_error = None;
        for attempt in 1..=$max_retries {
            match $test_fn().await {
                Ok(result) => break Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < $max_retries {
                        eprintln!("异步测试失败，第 {} 次重试...", attempt);
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempt)).await;
                    }
                }
            }
        }
        last_error.map(Err).unwrap_or(Ok(()))
    }};
}

/// 测试超时宏
#[macro_export]
macro_rules! with_timeout {
    ($duration:expr, $test_fn:expr) => {{
        tokio::time::timeout($duration, $test_fn)
            .await
            .map_err(|_| format!("测试超时: {:?}", $duration))
    }};
}

/// 并发测试宏
#[macro_export]
macro_rules! concurrent_test {
    ($test_fn:expr, $count:expr) => {{
        let mut handles = Vec::new();
        for i in 0..$count {
            let handle = tokio::spawn(async move { $test_fn(i).await });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        results
    }};
}

/// 性能测试宏
#[macro_export]
macro_rules! benchmark_test {
    ($test_fn:expr, $iterations:expr) => {{
        let start = std::time::Instant::now();
        for _ in 0..$iterations {
            $test_fn();
        }
        let duration = start.elapsed();
        let avg_duration = duration / $iterations;

        println!("基准测试结果:");
        println!("  总时间: {:?}", duration);
        println!("  迭代次数: {}", $iterations);
        println!("  平均时间: {:?}", avg_duration);

        (duration, avg_duration)
    }};
}

/// 异步性能测试宏
#[macro_export]
macro_rules! benchmark_async_test {
    ($test_fn:expr, $iterations:expr) => {{
        let start = std::time::Instant::now();
        for _ in 0..$iterations {
            $test_fn().await;
        }
        let duration = start.elapsed();
        let avg_duration = duration / $iterations;

        println!("异步基准测试结果:");
        println!("  总时间: {:?}", duration);
        println!("  迭代次数: {}", $iterations);
        println!("  平均时间: {:?}", avg_duration);

        (duration, avg_duration)
    }};
}

/// 内存使用测试宏
#[macro_export]
macro_rules! memory_test {
    ($test_fn:expr) => {{
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let get_memory_usage = || -> Result<usize, Box<dyn std::error::Error>> {
                let status = fs::read_to_string("/proc/self/status")?;
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return Ok(parts[1].parse::<usize>()? * 1024); // Convert KB to bytes
                        }
                    }
                }
                Err("无法找到内存使用信息".into())
            };

            let memory_before = get_memory_usage().unwrap_or(0);
            let result = $test_fn();
            let memory_after = get_memory_usage().unwrap_or(0);

            let memory_diff = memory_after.saturating_sub(memory_before);
            println!("内存使用测试结果:");
            println!("  测试前: {} bytes", memory_before);
            println!("  测试后: {} bytes", memory_after);
            println!("  内存增长: {} bytes", memory_diff);

            (result, memory_diff)
        }

        #[cfg(not(target_os = "linux"))]
        {
            println!("内存测试仅在Linux上支持");
            ($test_fn(), 0)
        }
    }};
}

/// 测试分组宏
#[macro_export]
macro_rules! test_group {
    ($group_name:expr, { $($test_name:ident: $test_fn:expr),* $(,)? }) => {
        mod $group_name {
            use super::*;

            $(
                #[tokio::test]
                async fn $test_name() {
                    println!("运行测试组 '{}' 中的测试: {}", stringify!($group_name), stringify!($test_name));
                    $test_fn().await;
                }
            )*
        }
    };
}

/// 条件测试宏
#[macro_export]
macro_rules! conditional_test {
    ($condition:expr, $test_fn:expr) => {
        if $condition {
            $test_fn();
        } else {
            println!("跳过测试，条件不满足: {}", stringify!($condition));
        }
    };
}

/// 平台特定测试宏
#[macro_export]
macro_rules! platform_test {
    (unix: $test_fn:expr) => {
        #[cfg(unix)]
        $test_fn();

        #[cfg(not(unix))]
        println!("跳过Unix特定测试");
    };

    (windows: $test_fn:expr) => {
        #[cfg(windows)]
        $test_fn();

        #[cfg(not(windows))]
        println!("跳过Windows特定测试");
    };

    (macos: $test_fn:expr) => {
        #[cfg(target_os = "macos")]
        $test_fn();

        #[cfg(not(target_os = "macos"))]
        println!("跳过macOS特定测试");
    };
}
