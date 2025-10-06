//! Agent context orchestration primitives (stage 2).

pub mod builder;
pub mod file_tracker;
pub mod summarizer;

pub use crate::agent::config::ContextBuilderConfig;
pub use builder::ContextBuilder;
pub use file_tracker::{FileContextTracker, FileOperationRecord};
pub use summarizer::{ConversationSummarizer, SummaryResult};
