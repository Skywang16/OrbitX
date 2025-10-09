use serde::{Deserialize, Serialize};

/// React runtime thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactRuntimeConfig {
    pub max_iterations: u32,
    pub max_consecutive_errors: u32,
    pub max_idle_rounds: u32,
}
