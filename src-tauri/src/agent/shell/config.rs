//! Shell 执行器配置

use std::time::Duration;

/// Shell 执行器配置
#[derive(Debug, Clone)]
pub struct ShellExecutorConfig {
    /// 默认超时时间
    pub default_timeout: Duration,
    /// 输出缓冲区大小（字节）
    pub output_buffer_size: usize,
    /// 最大并发后台命令数
    pub max_background_commands: usize,
    /// 已完成命令保留时间
    pub completed_retention: Duration,
    /// 命令最大长度
    pub max_command_length: usize,
    /// 最大超时时间
    pub max_timeout: Duration,
}

impl Default for ShellExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(120),
            output_buffer_size: 1024 * 1024, // 1MB
            max_background_commands: 10,
            completed_retention: Duration::from_secs(300), // 5 分钟
            max_command_length: 10 * 1024,                 // 10KB
            max_timeout: Duration::from_secs(600),         // 10 分钟
        }
    }
}
