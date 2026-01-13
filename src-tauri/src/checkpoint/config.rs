//! Checkpoint 配置系统

use std::time::Duration;

/// Checkpoint 系统配置
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// 最大文件大小（字节），超过此大小的文件不会被快照
    pub max_file_size: u64,

    /// 忽略的文件模式（glob 格式）
    pub ignored_patterns: Vec<String>,

    /// 最大 checkpoint 数量（超过后自动清理旧的）
    pub max_checkpoints: usize,

    /// 自动垃圾回收间隔
    pub gc_interval: Duration,

    /// 流式处理的缓冲区大小
    pub stream_buffer_size: usize,

    /// 并发处理文件的最大数量
    pub max_concurrent_files: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            max_file_size: 50 * 1024 * 1024, // 50MB
            ignored_patterns: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".git/**".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
            ],
            max_checkpoints: 100,
            gc_interval: Duration::from_secs(300), // 5分钟
            stream_buffer_size: 64 * 1024,         // 64KB
            max_concurrent_files: 10,
        }
    }
}

impl CheckpointConfig {
    /// 检查文件是否应该被忽略
    pub fn should_ignore_file(&self, file_path: &str) -> bool {
        for pattern in &self.ignored_patterns {
            if glob_match(pattern, file_path) {
                return true;
            }
        }
        false
    }

    /// 检查文件大小是否超过限制
    pub fn is_file_too_large(&self, size: u64) -> bool {
        size > self.max_file_size
    }
}

/// 简单的 glob 匹配实现
fn glob_match(pattern: &str, text: &str) -> bool {
    // 简化版本，实际应该使用 glob crate
    if let Some(prefix) = pattern.strip_suffix("/**") {
        text.starts_with(prefix)
    } else if pattern.contains('*') {
        // 简单的通配符匹配
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            false
        }
    } else {
        text == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_file() {
        let config = CheckpointConfig::default();

        assert!(config.should_ignore_file("node_modules/react/index.js"));
        assert!(config.should_ignore_file("target/debug/main"));
        assert!(config.should_ignore_file("test.log"));
        assert!(!config.should_ignore_file("src/main.rs"));
    }

    #[test]
    fn test_file_size_limit() {
        let config = CheckpointConfig::default();

        assert!(!config.is_file_too_large(1024)); // 1KB
        assert!(config.is_file_too_large(100 * 1024 * 1024)); // 100MB
    }
}
