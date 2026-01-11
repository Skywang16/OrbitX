#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub enabled: bool,
    pub auto_prune: bool,
    pub auto_compact: bool,

    pub prune_threshold: f32,
    pub compact_threshold: f32,
    pub max_messages_before_compact: usize,

    pub keep_recent_messages: usize,
    pub protected_tools: Vec<String>,

    pub summary_max_tokens: u32,
    pub summary_model: Option<String>,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_prune: true,
            auto_compact: true,

            prune_threshold: 0.85,
            compact_threshold: 0.90,
            max_messages_before_compact: 50,

            keep_recent_messages: 3,
            protected_tools: vec!["todoread".to_string(), "todowrite".to_string()],

            summary_max_tokens: 1024,
            summary_model: None,
        }
    }
}
