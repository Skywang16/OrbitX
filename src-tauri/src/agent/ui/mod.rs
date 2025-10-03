//! Agent UI persistence layer.
//! Responsible for recording task progress events for UI rendering,
//! independent from the agent's core execution persistence.

mod models;
mod persistence;

pub use models::{UiConversation, UiMessage, UiStep};
pub use persistence::AgentUiPersistence;
