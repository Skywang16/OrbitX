// 终端上下文管理模块

pub mod channel_manager;
pub mod channel_state;
pub mod commands;
pub mod context_registry;
pub mod context_service;
pub mod event_handler;
#[cfg(test)]
pub mod integration_test;
pub mod types;

pub use channel_manager::TerminalChannelManager;
pub use channel_state::TerminalChannelState;
pub use commands::TerminalContextState;
pub use context_registry::ActiveTerminalContextRegistry;
pub use context_service::{CacheStats, TerminalContextService};
pub use event_handler::{create_terminal_event_handler, TerminalEventHandler};
pub use types::*;
