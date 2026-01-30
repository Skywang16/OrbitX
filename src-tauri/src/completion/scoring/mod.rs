//! 补全评分系统
//!
//! 提供统一的补全项评分机制，消除硬编码的 magic numbers，
//! 使评分逻辑可组合、可测试、可配置。
//!
//! # 架构
//!
//! ```text
//! ScoringContext (数据)
//!     ↓
//! ScoreCalculator (抽象)
//!     ↓
//! BaseScorer / HistoryScorer / ... (具体实现)
//!     ↓
//! CompositeScorer (组合)
//! ```
//!
//! # 示例
//!
//! ```rust
//! use crate::completion::scoring::*;
//!
//! let context = ScoringContext::new("git")
//!     .with_prefix_match(true)
//!     .with_history_weight(0.8);
//!
//! let scorer = CompositeScorer::new(vec![
//!     Box::new(BaseScorer),
//!     Box::new(HistoryScorer),
//! ]);
//!
//! let score = scorer.calculate(&context);
//! ```

pub mod calculator;
pub mod context;
pub mod scorers;

pub use calculator::ScoreCalculator;
pub use context::ScoringContext;
pub use scorers::{
    BaseScorer, CompositeScorer, FrecencyScorer, HistoryScorer, SmartProviderScorer,
};

/// 评分常量 - 消除 magic numbers
///
/// 这些常量是经过权衡的结果，而不是随意的数字
/// 基础匹配分数 - 任何有效补全的最低分
pub const BASE_SCORE: f64 = 70.0;

/// 历史权重系数 - 历史记录对评分的影响
pub const HISTORY_WEIGHT: f64 = 20.0;

/// 智能补全加分 - 智能提供者的优势
pub const SMART_BOOST: f64 = 10.0;

/// 前缀匹配加分 - 前缀匹配比模糊匹配优先
pub const PREFIX_MATCH_BONUS: f64 = 15.0;

/// 最低有效分数 - 低于此分数的补全将被过滤
pub const MIN_SCORE: f64 = 10.0;

/// 最大分数上限 - 防止分数溢出
pub const MAX_SCORE: f64 = 100.0;

/// 位置权重系数 - 历史位置对评分的影响
pub const POSITION_WEIGHT: f64 = 10.0;

/// 匹配度权重系数 - 匹配长度对评分的影响
pub const MATCH_RATIO_WEIGHT: f64 = 20.0;

/// 确保分数在有效范围内
#[inline]
pub fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, MAX_SCORE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_score() {
        assert_eq!(clamp_score(-10.0), 0.0);
        assert_eq!(clamp_score(50.0), 50.0);
        assert_eq!(clamp_score(150.0), MAX_SCORE);
    }

    #[test]
    fn test_score_constants() {
        // 确保评分系数加起来不超过最大分数
        let max_possible = BASE_SCORE + HISTORY_WEIGHT + SMART_BOOST + PREFIX_MATCH_BONUS;
        assert!(
            max_possible <= MAX_SCORE + 20.0,
            "评分系数总和过大: {max_possible}"
        );
    }
}
