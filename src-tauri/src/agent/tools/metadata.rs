use std::time::Duration;

pub use crate::agent::config::{BackoffStrategy, RateLimitConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    FileRead,
    FileWrite,
    CodeAnalysis,
    Execution,
    Network,
    FileSystem,
    Terminal,
}

impl ToolCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "file_read",
            Self::FileWrite => "file_write",
            Self::CodeAnalysis => "code_analysis",
            Self::Execution => "execution",
            Self::Network => "network",
            Self::FileSystem => "filesystem",
            Self::Terminal => "terminal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ToolPriority {
    Critical = 0,
    Standard = 1,
    Expensive = 2,
}

impl ToolPriority {
    pub fn timeout_millis(&self) -> u64 {
        match self {
            Self::Critical => 5_000,
            Self::Standard => 30_000,
            Self::Expensive => 120_000,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Standard => "standard",
            Self::Expensive => "expensive",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub category: ToolCategory,
    pub priority: ToolPriority,
    pub custom_timeout: Option<Duration>,
    pub rate_limit: Option<RateLimitConfig>,
    pub requires_confirmation: bool,
    pub tags: Vec<String>,
    /// Key argument field name for summarization (e.g., "path" for file tools, "command" for shell)
    pub summary_key_arg: Option<&'static str>,
}

impl ToolMetadata {
    pub fn new(category: ToolCategory, priority: ToolPriority) -> Self {
        Self {
            category,
            priority,
            custom_timeout: None,
            rate_limit: None,
            requires_confirmation: false,
            tags: Vec::new(),
            summary_key_arg: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.custom_timeout = Some(timeout);
        self
    }

    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = Some(config);
        self
    }

    pub fn with_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_summary_key_arg(mut self, key_arg: &'static str) -> Self {
        self.summary_key_arg = Some(key_arg);
        self
    }

    pub fn effective_timeout(&self) -> Duration {
        self.custom_timeout
            .unwrap_or_else(|| Duration::from_millis(self.priority.timeout_millis()))
    }
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self::new(ToolCategory::FileRead, ToolPriority::Standard)
    }
}
