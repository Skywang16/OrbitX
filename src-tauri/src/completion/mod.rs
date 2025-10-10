//! 终端补全功能模块
//!
//! 提供智能的终端命令补全功能，包括：
//! - 文件路径补全
//! - 命令历史补全
//! - 系统命令补全
//! - 环境变量补全

pub mod commands;
pub mod context_analyzer;
pub mod engine;
pub mod error;
pub mod output_analyzer;
pub mod providers;
pub mod scoring;
pub mod smart_extractor;
pub mod smart_provider;
pub mod types;

pub use commands::*;
pub use engine::*;
pub use error::*;
pub use providers::*;
pub use types::*;
