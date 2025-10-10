//! Node.js 版本管理模块
//! 
//! 提供 Node.js 项目检测、版本管理器识别、版本切换等功能

pub mod commands;
pub mod detector;
pub mod types;

pub use commands::*;
pub use types::*;
