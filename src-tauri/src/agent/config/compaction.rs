//! Compaction configuration for tool result clearing.

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Number of recent messages to keep unmodified
    pub keep_recent_count: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            keep_recent_count: 3,
        }
    }
}
