use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionConfig {
    pub enabled: bool,
    pub min_messages: u32,
    pub max_unsummarized_messages: u32,
    pub keep_recent_messages: u32,
    pub max_summary_chars: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_messages: 20,
            max_unsummarized_messages: 30,
            keep_recent_messages: 8,
            max_summary_chars: 8_000,
        }
    }
}
