/*!
 * 终端上下文管理模块
 *
 * 提供统一的终端上下文管理功能，包括：
 * - 活跃终端状态跟踪
 * - 终端上下文服务
 * - 事件处理和传播
 * - Tauri 命令接口
 */

pub mod commands;
pub mod context_registry;
pub mod context_service;
pub mod event_handler;
pub mod integration_test;
pub mod types;

pub use commands::TerminalContextState;
pub use context_registry::ActiveTerminalContextRegistry;
pub use context_service::{CacheStats, CachedContext, TerminalContextService};
pub use event_handler::{create_terminal_event_handler, EventHandlerStatus, TerminalEventHandler};
pub use types::*;
