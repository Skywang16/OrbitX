//! 具体评分器实现
//!
//! 提供各种评分策略的具体实现，消除硬编码的 magic numbers

use super::calculator::ScoreCalculator;
use super::context::ScoringContext;
use super::*;

/// 基础评分器
///
/// 根据前缀匹配和匹配度计算基础分数
pub struct BaseScorer;

impl ScoreCalculator for BaseScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // 前缀匹配是最基础的要求
        if context.is_prefix_match {
            score += BASE_SCORE;

            // 匹配度加分：输入越接近完整补全，分数越高
            let match_ratio = context.match_ratio();
            score += match_ratio * MATCH_RATIO_WEIGHT;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "base"
    }
}

/// 历史评分器
///
/// 根据历史记录的频率和位置计算分数
pub struct HistoryScorer;

impl ScoreCalculator for HistoryScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // 历史权重：反映使用频率
        score += context.history_weight * HISTORY_WEIGHT;

        // 位置加分：越新的命令分数越高
        if let Some(position) = context.history_position {
            let position_factor = (1000 - position.min(1000)) as f64 / 1000.0;
            score += position_factor * POSITION_WEIGHT;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "history"
    }
}

/// 智能补全评分器
///
/// 为智能提供者提供的补全加分
pub struct SmartProviderScorer;

impl ScoreCalculator for SmartProviderScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // 智能补全默认加分
        if context.source.as_deref() == Some("smart") {
            score += SMART_BOOST;
        }

        // 前缀匹配额外加分
        if context.is_prefix_match {
            score += PREFIX_MATCH_BONUS;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "smart"
    }
}

/// 组合评分器
///
/// 组合多个评分器的结果，支持可组合的评分策略
pub struct CompositeScorer {
    scorers: Vec<Box<dyn ScoreCalculator>>,
}

impl CompositeScorer {
    /// 创建新的组合评分器
    pub fn new(scorers: Vec<Box<dyn ScoreCalculator>>) -> Self {
        Self { scorers }
    }

    /// 创建默认的组合评分器（用于通用场景）
    pub fn default_composite() -> Self {
        Self::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(SmartProviderScorer),
        ])
    }
}

impl ScoreCalculator for CompositeScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let total: f64 = self
            .scorers
            .iter()
            .map(|scorer| scorer.calculate(context))
            .sum();

        clamp_score(total)
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_scorer() {
        let scorer = BaseScorer;

        // 前缀匹配，高匹配度
        let ctx1 = ScoringContext::new("git status", "git status").with_prefix_match(true);
        let score1 = scorer.calculate(&ctx1);
        assert!(score1 > BASE_SCORE, "完全匹配应该有额外加分");

        // 前缀匹配，低匹配度
        let ctx2 = ScoringContext::new("git", "git status").with_prefix_match(true);
        let score2 = scorer.calculate(&ctx2);
        assert!(
            score2 >= BASE_SCORE && score2 < score1,
            "部分匹配分数应该更低"
        );

        // 非前缀匹配
        let ctx3 = ScoringContext::new("status", "git status").with_prefix_match(false);
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, 0.0, "非前缀匹配应该得 0 分");
    }

    #[test]
    fn test_history_scorer() {
        let scorer = HistoryScorer;

        // 高权重，最新位置
        let ctx1 = ScoringContext::new("git", "git status")
            .with_history_weight(1.0)
            .with_history_position(0);
        let score1 = scorer.calculate(&ctx1);
        assert!(score1 > HISTORY_WEIGHT, "最新且高频应该有高分");

        // 低权重，旧位置
        let ctx2 = ScoringContext::new("git", "git status")
            .with_history_weight(0.1)
            .with_history_position(999);
        let score2 = scorer.calculate(&ctx2);
        assert!(score2 < score1, "旧命令分数应该更低");

        // 无历史数据
        let ctx3 = ScoringContext::new("git", "git status");
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, 0.0, "无历史数据应该得 0 分");
    }

    #[test]
    fn test_smart_provider_scorer() {
        let scorer = SmartProviderScorer;

        // 智能提供者 + 前缀匹配
        let ctx1 = ScoringContext::new("git", "git status")
            .with_source("smart")
            .with_prefix_match(true);
        let score1 = scorer.calculate(&ctx1);
        assert!(score1 > SMART_BOOST, "智能+前缀应该有额外加分");

        // 非智能提供者
        let ctx2 = ScoringContext::new("git", "git status")
            .with_source("history")
            .with_prefix_match(true);
        let score2 = scorer.calculate(&ctx2);
        assert_eq!(score2, PREFIX_MATCH_BONUS, "应该只有前缀加分");

        // 智能提供者但非前缀
        let ctx3 = ScoringContext::new("git", "git status")
            .with_source("smart")
            .with_prefix_match(false);
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, SMART_BOOST, "应该只有智能加分");
    }

    #[test]
    fn test_composite_scorer() {
        let scorer = CompositeScorer::default_composite();

        // 完美场景：前缀匹配 + 高历史权重 + 智能提供者
        let ctx = ScoringContext::new("git", "git status")
            .with_prefix_match(true)
            .with_history_weight(1.0)
            .with_history_position(0)
            .with_source("smart");

        let score = scorer.calculate(&ctx);
        assert!(score > BASE_SCORE, "组合评分应该大于基础分");
        assert!(score <= MAX_SCORE, "分数不应超过上限");
    }

    #[test]
    fn test_score_clamping() {
        let scorer = CompositeScorer::default_composite();

        // 创建极端场景，确保分数不会溢出
        let ctx = ScoringContext::new("test", "test")
            .with_prefix_match(true)
            .with_history_weight(1.0)
            .with_history_position(0)
            .with_source("smart");

        let score = scorer.calculate(&ctx);
        assert!(score >= 0.0 && score <= MAX_SCORE, "分数应该在有效范围内");
    }

    #[test]
    fn test_empty_composite_scorer() {
        let scorer = CompositeScorer::new(vec![]);
        let ctx = ScoringContext::new("test", "test");

        assert_eq!(scorer.calculate(&ctx), 0.0, "空组合应该返回 0 分");
    }

    #[test]
    fn test_custom_composite() {
        // 只使用基础和历史评分器
        let scorer = CompositeScorer::new(vec![Box::new(BaseScorer), Box::new(HistoryScorer)]);

        let ctx = ScoringContext::new("git", "git status")
            .with_prefix_match(true)
            .with_history_weight(0.5);

        let score = scorer.calculate(&ctx);
        assert!(score > 0.0, "自定义组合应该正常工作");
    }
}
