//! Terminal Mux - 核心终端多路复用器
//!
//! 提供统一的终端会话管理、事件通知和PTY I/O处理

pub mod config;

pub mod io_handler;
pub mod pane;
pub mod performance_monitor;
pub mod singleton;
// Note: tauri_integration module removed - event handling now unified in terminal::event_handler
pub mod terminal_mux;
pub mod types;

pub use config::*;

pub use io_handler::*;
pub use pane::*;
pub use performance_monitor::*;
pub use singleton::*;
pub use terminal_mux::*;
pub use types::*;
