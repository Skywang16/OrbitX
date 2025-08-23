//! Shell Integration - 完整的Shell集成系统
//!
//! 支持多种Shell的集成，包括命令跟踪、CWD同步、窗口标题更新等功能

pub mod commands;
pub mod integration;
pub mod osc_parser;
pub mod script_generator;

pub use commands::*;
pub use integration::*;
pub use osc_parser::*;
pub use script_generator::*;
