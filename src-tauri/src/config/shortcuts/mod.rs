/*!
 * 快捷键配置系统模块
 *
 * 提供快捷键的验证、冲突检测、平台适配和管理功能。
 */

pub mod commands;
pub mod conflict_detector;
pub mod manager;
pub mod platform_adapter;
pub mod validator;

// 重新导出核心类型和函数
pub use commands::*;
pub use conflict_detector::*;
pub use manager::*;
pub use platform_adapter::*;
pub use validator::*;
