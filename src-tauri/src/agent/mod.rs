pub mod agents;
pub mod command_system;
/// Agent module - provides complete Agent system
pub mod config;
pub mod error;
pub mod prompt;
pub mod skill;
pub mod types;

pub mod common; // Common utilities and templates
pub mod compaction; // Context engineering: Prune/Compact/checkpoint loading
pub mod context; // Thread context tracker and summarizer
pub mod core; // Agent run executor core (executor only, no tool-related)
pub mod mcp; // MCP adapter
pub mod permissions; // settings.json permissions (allow/deny/ask)
pub mod persistence; // Persistence and repository abstraction
pub mod react; // ReAct strategy and parsing
pub mod rollout; // Rollout-first thread persistence and replay
pub mod shell; // Shell execution module
pub mod state; // Agent runtime state and errors
pub mod subagent; // First-class subagent records
pub mod terminal; // Agent terminal subsystem
pub mod tools; // Tool interface and built-in tools
pub mod utils; // Utility functions
pub mod workspace_changes; // Workspace change ledger (user/external change injection)
pub use config::*;
pub use error::*;
pub use types::*;

pub use core::AgentRunExecutor;
pub use tools::{ToolExecutionLogger, ToolRegistry};
