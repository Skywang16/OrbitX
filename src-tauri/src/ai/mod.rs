/*!
 * AI集成模块
 *
 * 提供AI功能的核心实现，包括：
 * - 模型配置管理
 * - 统一的AI客户端
 * - 缓存管理
 * - 命令处理
 */

pub mod commands;
pub mod context;
pub mod service;
pub mod tool;
pub mod types;

// 重新导出主要类型和功能
pub use commands::*;
pub use context::*;
pub use service::*;
pub use types::*;

// 重新导出统一的错误处理类型
pub use crate::utils::error::{AppError, AppResult};
