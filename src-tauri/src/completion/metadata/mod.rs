//! 命令元数据模块
//!
//! 提供配置驱动的命令元数据系统，替代硬编码的命令列表
//!

pub mod builtin;
pub mod command_spec;
pub mod registry;

pub use command_spec::CommandSpec;
pub use registry::CommandRegistry;
