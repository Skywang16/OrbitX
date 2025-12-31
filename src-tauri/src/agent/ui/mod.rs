//! Agent UI persistence layer.
//! Responsible for recording task progress events for UI rendering,
//! independent from the agent's core execution persistence.

mod models;

pub use models::{UiMessageImage, UiStep};
