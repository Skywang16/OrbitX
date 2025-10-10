//! 命令元数据模块
//!
//! 提供配置驱动的命令元数据系统，替代硬编码的命令列表
//!
//! # 设计理念
//!
//! 遵循 Linus 的原则：
//! - 数据驱动而非代码驱动
//! - 易于扩展和维护
//! - 用户可以添加自定义命令

pub mod builtin;
pub mod command_spec;
pub mod registry;

pub use command_spec::CommandSpec;
pub use registry::CommandRegistry;
