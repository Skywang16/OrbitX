//! Agent persistence layer rebuilt for the new context-aware architecture.
//! Provides strongly-typed repositories that own the lifecycle of conversation,
//! execution and tool tracking entities defined in `sql/07_agent_context_architecture.sql`.

mod manager;
pub mod models;
pub mod repositories;
mod util;

pub use manager::AgentPersistence;
pub use models::*;
pub use repositories::*;

pub(crate) use util::*; // Internal helpers shared across repositories.
