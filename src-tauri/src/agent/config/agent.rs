use serde::{Deserialize, Serialize};

/// Minimal agent runtime configuration used by thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_react_num: u32,
    pub max_react_error_streak: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_react_num: 100,
            max_react_error_streak: 5,
        }
    }
}
