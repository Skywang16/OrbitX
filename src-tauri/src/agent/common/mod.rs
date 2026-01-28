//! Common/shared utilities for Agent module

// Re-export prompt utilities
pub use crate::agent::prompt::{BuiltinPrompts, PromptBuilder};

pub mod llm_text;
pub mod text;

pub use text::{truncate_chars, truncate_chars_no_ellipsis};
