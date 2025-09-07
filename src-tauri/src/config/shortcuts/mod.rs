/*!
 * 全新快捷键系统
 *
 * 采用配置驱动设计，支持：
 * - 统一的快捷键管理
 * - 动态功能映射  
 * - 冲突检测和验证
 * - 运行时配置更新
 */

pub mod actions;
pub mod commands;
pub mod core;
pub mod types;

// 重新导出核心模块
pub use actions::*;
pub use commands::*;
pub use core::*;
pub use types::*;
