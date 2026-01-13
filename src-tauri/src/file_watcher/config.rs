use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWatcherConfig {
    pub enable_fs_watcher: bool,
    pub enable_git_watcher: bool,
    pub debounce_ms: u64,
    pub throttle_ms: u64,
    pub ignore_patterns: Vec<String>,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            enable_fs_watcher: true,
            enable_git_watcher: true,
            debounce_ms: 1000,
            throttle_ms: 2000,
            ignore_patterns: vec![
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                "*.log".to_string(),
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
            ],
        }
    }
}
