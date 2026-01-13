//! 评分上下文
//!
//! 封装评分计算所需的所有数据，遵循"数据与逻辑分离"原则

use std::path::PathBuf;

/// 评分上下文
///
/// 包含评分器需要的所有信息，而不混入评分逻辑
#[derive(Debug, Clone)]
pub struct ScoringContext {
    /// 用户输入的完整文本
    pub input: String,

    /// 光标位置
    pub cursor_position: usize,

    /// 当前补全项的文本
    pub item_text: String,

    /// 是否为前缀匹配
    pub is_prefix_match: bool,

    /// 历史权重（0.0 到 1.0）
    pub history_weight: f64,

    /// 历史位置索引（越小越新）
    pub history_position: Option<usize>,

    /// 当前工作目录
    pub working_directory: Option<PathBuf>,

    /// 是否在 Git 仓库中
    pub in_git_repo: bool,

    /// 补全来源（如 "history", "filesystem", "smart"）
    pub source: Option<String>,

    /// 命令使用频率（用于 Frecency 算法）
    pub frequency: Option<usize>,

    /// 最后使用时间（秒级时间戳）
    pub last_used_timestamp: Option<u64>,
}

impl ScoringContext {
    /// 创建新的评分上下文
    pub fn new(input: impl Into<String>, item_text: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            cursor_position: 0,
            item_text: item_text.into(),
            is_prefix_match: false,
            history_weight: 0.0,
            history_position: None,
            working_directory: None,
            in_git_repo: false,
            source: None,
            frequency: None,
            last_used_timestamp: None,
        }
    }

    /// 设置光标位置
    pub fn with_cursor_position(mut self, pos: usize) -> Self {
        self.cursor_position = pos;
        self
    }

    /// 设置是否前缀匹配
    pub fn with_prefix_match(mut self, is_match: bool) -> Self {
        self.is_prefix_match = is_match;
        self
    }

    /// 设置历史权重
    pub fn with_history_weight(mut self, weight: f64) -> Self {
        self.history_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// 设置历史位置
    pub fn with_history_position(mut self, position: usize) -> Self {
        self.history_position = Some(position);
        self
    }

    /// 设置工作目录
    pub fn with_working_directory(mut self, path: PathBuf) -> Self {
        self.working_directory = Some(path);
        self
    }

    /// 设置 Git 仓库状态
    pub fn with_git_repo(mut self, in_repo: bool) -> Self {
        self.in_git_repo = in_repo;
        self
    }

    /// 设置补全来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// 设置使用频率
    pub fn with_frequency(mut self, frequency: usize) -> Self {
        self.frequency = Some(frequency);
        self
    }

    /// 设置最后使用时间戳
    pub fn with_last_used_timestamp(mut self, timestamp: u64) -> Self {
        self.last_used_timestamp = Some(timestamp);
        self
    }

    /// 计算匹配度（输入长度 / 补全文本长度）
    pub fn match_ratio(&self) -> f64 {
        if self.item_text.is_empty() {
            return 0.0;
        }
        self.input.len() as f64 / self.item_text.len() as f64
    }

    /// 检查是否为完全匹配
    pub fn is_exact_match(&self) -> bool {
        self.input == self.item_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = ScoringContext::new("git st", "git status")
            .with_cursor_position(6)
            .with_prefix_match(true)
            .with_history_weight(0.8)
            .with_history_position(5)
            .with_source("history");

        assert_eq!(ctx.input, "git st");
        assert_eq!(ctx.item_text, "git status");
        assert_eq!(ctx.cursor_position, 6);
        assert!(ctx.is_prefix_match);
        assert_eq!(ctx.history_weight, 0.8);
        assert_eq!(ctx.history_position, Some(5));
        assert_eq!(ctx.source, Some("history".to_string()));
    }

    #[test]
    fn test_match_ratio() {
        let ctx = ScoringContext::new("git", "git status");
        assert!((ctx.match_ratio() - 3.0 / 10.0).abs() < 0.001);

        let ctx_exact = ScoringContext::new("git", "git");
        assert!((ctx_exact.match_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_is_exact_match() {
        let ctx = ScoringContext::new("git", "git");
        assert!(ctx.is_exact_match());

        let ctx2 = ScoringContext::new("git", "git status");
        assert!(!ctx2.is_exact_match());
    }

    #[test]
    fn test_history_weight_clamping() {
        let ctx = ScoringContext::new("test", "test").with_history_weight(1.5);
        assert_eq!(ctx.history_weight, 1.0);

        let ctx2 = ScoringContext::new("test", "test").with_history_weight(-0.5);
        assert_eq!(ctx2.history_weight, 0.0);
    }
}
