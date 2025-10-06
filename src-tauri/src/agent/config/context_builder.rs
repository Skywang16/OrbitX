//! Context builder configuration.

#[derive(Debug, Clone)]
pub struct ContextBuilderConfig {
    pub recent_file_window: usize,
    pub max_file_context_chars: usize,
    pub include_stale_hints: bool,
}

impl Default for ContextBuilderConfig {
    fn default() -> Self {
        Self {
            recent_file_window: 5,
            max_file_context_chars: 2048,
            include_stale_hints: true,
        }
    }
}
