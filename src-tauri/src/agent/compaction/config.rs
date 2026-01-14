use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionConfig {
    pub enabled: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

