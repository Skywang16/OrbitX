// 终端上下文管理模块

pub mod channel_manager;
pub mod channel_state;
pub mod commands;
pub mod context_registry;
pub mod context_service;
pub mod error;
pub mod event_handler;
#[cfg(test)]
pub mod integration_test;
pub mod replay;
pub mod types;

pub use channel_manager::TerminalChannelManager;
pub use channel_state::TerminalChannelState;
pub use commands::TerminalContextState;
pub use context_registry::ActiveTerminalContextRegistry;
pub use context_service::{CacheStats, TerminalContextService};
pub use error::{
    ContextRegistryError, ContextRegistryResult, ContextServiceError, ContextServiceResult,
    EventHandlerError, EventHandlerResult, ReplayError, ReplayResult, TerminalError,
    TerminalResult, TerminalValidationError, TerminalValidationResult,
};
pub use event_handler::{create_terminal_event_handler, TerminalEventHandler};
pub use types::*;

// 从统一events模块导出Context事件
pub use crate::events::TerminalContextEvent;
