//! Compaction (history summarization) configuration.

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub trigger_threshold: usize,
    pub keep_recent_count: usize,
    pub summary_max_tokens: u32,
    pub summary_temperature: f32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            trigger_threshold: 15,
            keep_recent_count: 3,
            summary_max_tokens: 512,
            summary_temperature: 0.3,
        }
    }
}
