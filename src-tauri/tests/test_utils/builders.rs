/*!
 * 测试数据构建器
 * 
 * 提供流式API来构建测试数据，减少重复的测试数据创建代码
 */

use std::collections::HashMap;
use std::time::Duration;

/// 通用构建器trait
pub trait Builder<T> {
    fn build(self) -> T;
}

/// 测试配置构建器
#[derive(Debug, Clone)]
pub struct TestConfigBuilder {
    timeout: Duration,
    retry_count: usize,
    cleanup_on_drop: bool,
    parallel: bool,
    verbose: bool,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retry_count: 3,
            cleanup_on_drop: true,
            parallel: false,
            verbose: false,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn retry_count(mut self, count: usize) -> Self {
        self.retry_count = count;
        self
    }

    pub fn cleanup_on_drop(mut self, cleanup: bool) -> Self {
        self.cleanup_on_drop = cleanup;
        self
    }

    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

impl Builder<crate::TestConfig> for TestConfigBuilder {
    fn build(self) -> crate::TestConfig {
        crate::TestConfig {
            timeout: self.timeout,
            retry_count: self.retry_count,
            cleanup_on_drop: self.cleanup_on_drop,
        }
    }
}

/// 错误测试数据构建器
#[derive(Debug, Clone)]
pub struct ErrorTestBuilder {
    message: String,
    error_code: Option<String>,
    context: HashMap<String, String>,
    recoverable: bool,
}

impl ErrorTestBuilder {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            error_code: None,
            context: HashMap::new(),
            recoverable: true,
        }
    }

    pub fn error_code(mut self, code: &str) -> Self {
        self.error_code = Some(code.to_string());
        self
    }

    pub fn context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }

    pub fn recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = recoverable;
        self
    }

    pub fn build_terminal_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Terminal(self.message)
    }

    pub fn build_io_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Io(self.message)
    }

    pub fn build_internal_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Internal(self.message)
    }

    pub fn build_ai_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::AI {
            message: self.message,
            model_id: None,
            error_code: self.error_code,
        }
    }

    pub fn build_completion_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Completion {
            message: self.message,
            provider: None,
            error_code: self.error_code,
        }
    }

    pub fn build_shell_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Shell {
            message: self.message,
            shell_path: None,
            error_code: self.error_code,
        }
    }

    pub fn build_mux_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Mux {
            message: self.message,
            pane_id: None,
            error_code: self.error_code,
        }
    }

    pub fn build_window_error(self) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::WindowOperation {
            message: self.message,
            window_id: None,
            error_code: self.error_code,
        }
    }

    pub fn build_config_error(self, key: &str) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Configuration {
            key: key.to_string(),
            message: self.message,
        }
    }

    pub fn build_validation_error(self, field: &str) -> terminal_lib::utils::error::AppError {
        terminal_lib::utils::error::AppError::Validation {
            field: field.to_string(),
            message: self.message,
        }
    }
}

/// 面板配置构建器
#[derive(Debug, Clone)]
pub struct PaneConfigBuilder {
    name: String,
    rows: u16,
    cols: u16,
    pixel_width: u16,
    pixel_height: u16,
    shell: Option<String>,
    working_dir: Option<String>,
    env_vars: HashMap<String, String>,
}

impl PaneConfigBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            rows: 24,
            cols: 80,
            pixel_width: 640,
            pixel_height: 480,
            shell: None,
            working_dir: None,
            env_vars: HashMap::new(),
        }
    }

    pub fn size(mut self, rows: u16, cols: u16) -> Self {
        self.rows = rows;
        self.cols = cols;
        self
    }

    pub fn pixel_size(mut self, width: u16, height: u16) -> Self {
        self.pixel_width = width;
        self.pixel_height = height;
        self
    }

    pub fn shell(mut self, shell: &str) -> Self {
        self.shell = Some(shell.to_string());
        self
    }

    pub fn working_dir(mut self, dir: &str) -> Self {
        self.working_dir = Some(dir.to_string());
        self
    }

    pub fn env_var(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    pub fn small() -> Self {
        Self::new("小面板").size(20, 60)
    }

    pub fn medium() -> Self {
        Self::new("中面板").size(30, 100)
    }

    pub fn large() -> Self {
        Self::new("大面板").size(40, 120)
    }

    pub fn build_pty_size(self) -> terminal_lib::mux::PtySize {
        terminal_lib::mux::PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
        }
    }
}

/// 测试数据构建器
#[derive(Debug, Clone)]
pub struct TestDataBuilder {
    content: String,
    encoding: String,
    line_ending: String,
    size: usize,
}

impl TestDataBuilder {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            encoding: "utf-8".to_string(),
            line_ending: "\n".to_string(),
            size: 0,
        }
    }

    pub fn content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self.size = content.len();
        self
    }

    pub fn line(mut self, line: &str) -> Self {
        if !self.content.is_empty() {
            self.content.push_str(&self.line_ending);
        }
        self.content.push_str(line);
        self.size = self.content.len();
        self
    }

    pub fn lines(mut self, lines: &[&str]) -> Self {
        for line in lines {
            self = self.line(line);
        }
        self
    }

    pub fn repeat_line(mut self, line: &str, count: usize) -> Self {
        for _ in 0..count {
            self = self.line(line);
        }
        self
    }

    pub fn encoding(mut self, encoding: &str) -> Self {
        self.encoding = encoding.to_string();
        self
    }

    pub fn line_ending(mut self, ending: &str) -> Self {
        self.line_ending = ending.to_string();
        self
    }

    pub fn random_content(mut self, size: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t"
            .chars()
            .collect();
        
        let content: String = (0..size)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();
        
        self.content = content;
        self.size = size;
        self
    }

    pub fn large_content(self, size: usize) -> Self {
        self.repeat_line(&"A".repeat(100), size / 100)
    }

    pub fn build_string(self) -> String {
        self.content
    }

    pub fn build_bytes(self) -> Vec<u8> {
        self.content.into_bytes()
    }

    pub fn build_with_newline(mut self) -> Vec<u8> {
        self.content.push_str(&self.line_ending);
        self.content.into_bytes()
    }
}

/// 性能测试配置构建器
#[derive(Debug, Clone)]
pub struct PerformanceTestBuilder {
    iterations: usize,
    warmup_iterations: usize,
    timeout: Duration,
    memory_limit: Option<usize>,
    cpu_limit: Option<f64>,
}

impl PerformanceTestBuilder {
    pub fn new() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
            timeout: Duration::from_secs(60),
            memory_limit: None,
            cpu_limit: None,
        }
    }

    pub fn iterations(mut self, count: usize) -> Self {
        self.iterations = count;
        self
    }

    pub fn warmup(mut self, count: usize) -> Self {
        self.warmup_iterations = count;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn memory_limit(mut self, limit_bytes: usize) -> Self {
        self.memory_limit = Some(limit_bytes);
        self
    }

    pub fn cpu_limit(mut self, limit_percent: f64) -> Self {
        self.cpu_limit = Some(limit_percent);
        self
    }

    pub fn quick() -> Self {
        Self::new().iterations(10).warmup(2).timeout(Duration::from_secs(5))
    }

    pub fn standard() -> Self {
        Self::new().iterations(100).warmup(10).timeout(Duration::from_secs(30))
    }

    pub fn intensive() -> Self {
        Self::new().iterations(1000).warmup(50).timeout(Duration::from_secs(300))
    }
}

/// 并发测试配置构建器
#[derive(Debug, Clone)]
pub struct ConcurrencyTestBuilder {
    thread_count: usize,
    operations_per_thread: usize,
    timeout: Duration,
    barrier_sync: bool,
}

impl ConcurrencyTestBuilder {
    pub fn new() -> Self {
        Self {
            thread_count: 4,
            operations_per_thread: 10,
            timeout: Duration::from_secs(30),
            barrier_sync: false,
        }
    }

    pub fn threads(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    pub fn operations_per_thread(mut self, count: usize) -> Self {
        self.operations_per_thread = count;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn barrier_sync(mut self, sync: bool) -> Self {
        self.barrier_sync = sync;
        self
    }

    pub fn light() -> Self {
        Self::new().threads(2).operations_per_thread(5)
    }

    pub fn moderate() -> Self {
        Self::new().threads(4).operations_per_thread(10)
    }

    pub fn heavy() -> Self {
        Self::new().threads(8).operations_per_thread(20)
    }
}
