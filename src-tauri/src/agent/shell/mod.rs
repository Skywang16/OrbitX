//! Agent Shell 执行模块
//!
//! 提供 Agent 专用的 Shell 命令执行功能，支持：
//! - 同步/异步执行
//! - 后台运行
//! - 超时控制
//! - 进程管理

mod buffer;
mod config;
mod error;
mod executor;
mod types;

pub use buffer::OutputRingBuffer;
pub use config::ShellExecutorConfig;
pub use error::ShellError;
pub use executor::AgentShellExecutor;
pub use types::*;
