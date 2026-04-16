//! Reasoning/Thinking context types for multi-provider support.
//!
//! Based on opencode-dev's design: unified ReasoningPart with provider-specific metadata.

use chrono::Utc;
use serde::{Deserialize, Serialize};

// Re-export the single metadata type to avoid duplication
pub use crate::llm::anthropic_types::ReasoningBlockMetadata as ReasoningMetadata;

/// Unified reasoning part that works across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningPart {
    pub id: String,
    pub thread_id: i64,
    pub message_id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ReasoningMetadata>,
    pub time: ReasoningTime,
}

/// Timing information for reasoning parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningTime {
    pub start: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

impl ReasoningPart {
    pub fn new(id: String, thread_id: i64, message_id: String) -> Self {
        Self {
            id,
            thread_id,
            message_id,
            text: String::new(),
            metadata: None,
            time: ReasoningTime {
                start: Utc::now().timestamp_millis(),
                end: None,
            },
        }
    }

    pub fn complete(&mut self) {
        self.time.end = Some(Utc::now().timestamp_millis());
        // Trim in place to avoid allocation
        let trimmed_len = self.text.trim_end().len();
        self.text.truncate(trimmed_len);
    }

    pub fn is_streaming(&self) -> bool {
        self.time.end.is_none()
    }

    pub fn set_openai_metadata(&mut self, item_id: String, encrypted_content: Option<String>) {
        let m = self.metadata.get_or_insert_with(ReasoningMetadata::default);
        m.item_id = Some(item_id);
        m.encrypted_content = encrypted_content;
        m.provider = Some("openai".to_string());
    }

    pub fn set_anthropic_metadata(&mut self, signature: Option<String>) {
        let m = self.metadata.get_or_insert_with(ReasoningMetadata::default);
        m.signature = signature;
        m.provider = Some("anthropic".to_string());
    }

    pub fn openai_item_id(&self) -> Option<&str> {
        self.metadata.as_ref().and_then(|m| m.item_id.as_deref())
    }

    pub fn anthropic_signature(&self) -> Option<&str> {
        self.metadata.as_ref().and_then(|m| m.signature.as_deref())
    }
}
