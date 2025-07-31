/*!
 * AI集成模块 - 简化版
 *
 * 提供AI功能的核心实现，包括：
 * - 模型配置管理
 * - 统一的AI客户端
 * - 简化的缓存管理
 * - 上下文管理
 * - 提示词引擎
 */

pub mod adapter_manager;
pub mod cache;
pub mod client;
pub mod command_processor;
pub mod commands;
pub mod config;
pub mod context_manager;
pub mod prompt_engine;
pub mod types;

// 重新导出主要类型和功能
pub use adapter_manager::*;
pub use cache::{
    AICacheStats, CacheManager, CacheMonitorStats, CachePerformanceMetrics, CacheStrategy,
    SimpleCache,
};
pub use client::*;
pub use command_processor::*;
pub use commands::*;
pub use config::*;
pub use context_manager::*;
pub use prompt_engine::*;
pub use types::*;

// 重新导出统一的错误处理类型
pub use crate::utils::error::{AppError, AppResult};

// 为了向后兼容性，保留一些旧的导出
pub use client::{AIAdapter, AIClient as UnifiedAIAdapter, AIClient as CustomAdapter};
