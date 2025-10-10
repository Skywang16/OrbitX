//! 评分计算器接口
//!
//! 定义统一的评分接口，支持不同的评分策略

use super::context::ScoringContext;

/// 评分计算器
///
/// 所有评分器都实现此 trait，保持一致的接口
pub trait ScoreCalculator: Send + Sync {
    /// 计算给定上下文的分数
    ///
    /// # 参数
    /// - `context`: 评分上下文，包含所有评分所需信息
    ///
    /// # 返回
    /// 分数值（通常在 0.0 到 100.0 之间）
    fn calculate(&self, context: &ScoringContext) -> f64;

    /// 评分器名称（用于调试和日志）
    fn name(&self) -> &'static str {
        "unknown"
    }
}

/// 为闭包实现 ScoreCalculator
///
/// 允许使用简单的函数作为评分器
impl<F> ScoreCalculator for F
where
    F: Fn(&ScoringContext) -> f64 + Send + Sync,
{
    fn calculate(&self, context: &ScoringContext) -> f64 {
        self(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestScorer {
        fixed_score: f64,
    }

    impl ScoreCalculator for TestScorer {
        fn calculate(&self, _context: &ScoringContext) -> f64 {
            self.fixed_score
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_scorer_trait() {
        let scorer = TestScorer { fixed_score: 75.0 };
        let ctx = ScoringContext::new("test", "test item");

        assert_eq!(scorer.calculate(&ctx), 75.0);
        assert_eq!(scorer.name(), "test");
    }

    #[test]
    fn test_closure_scorer() {
        let scorer = |ctx: &ScoringContext| -> f64 {
            if ctx.is_prefix_match {
                100.0
            } else {
                50.0
            }
        };

        let ctx1 = ScoringContext::new("test", "test").with_prefix_match(true);
        let ctx2 = ScoringContext::new("test", "other").with_prefix_match(false);

        assert_eq!(scorer.calculate(&ctx1), 100.0);
        assert_eq!(scorer.calculate(&ctx2), 50.0);
    }
}
