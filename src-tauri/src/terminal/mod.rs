// 终端上下文管理模块

pub mod commands;
pub mod context_registry;
pub mod context_service;
pub mod event_handler;
pub mod integration_test;
pub mod types;
pub mod channel_manager;
pub mod channel_state;

pub use commands::TerminalContextState;
pub use context_registry::ActiveTerminalContextRegistry;
pub use context_service::{CacheStats, CachedContext, TerminalContextService};
pub use event_handler::{create_terminal_event_handler, TerminalEventHandler};
pub use types::*;
pub use channel_manager::TerminalChannelManager;
pub use channel_state::TerminalChannelState;
