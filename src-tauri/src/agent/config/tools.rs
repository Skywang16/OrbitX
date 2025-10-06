use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64 },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_calls: u32,
    pub window_secs: u64,
    pub backoff: BackoffStrategy,
}
