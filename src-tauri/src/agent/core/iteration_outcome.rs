/*!
 * Iteration Outcome - ReAct 循环的迭代结果分类
 *
 * 这是架构重构的核心：用明确的数据结构替代隐式的判断逻辑。
 *
 * 设计原则：
 * 1. 消除特殊情况：只有三种明确的状态
 * 2. 数据驱动决策：outcome 决定是否继续循环
 * 3. 职责单一：只负责分类，不负责执行
 */

use crate::llm::types::LLMToolCall;
use serde::{Deserialize, Serialize};

/// 迭代结果：LLM 响应后的明确分类
///
/// 这个枚举消除了原来的所有隐式判断：
/// - 不再判断 `visible.is_empty()`
/// - 不再猜测 "有 thinking 算不算完成"
/// - 不再依赖 magic number
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IterationOutcome {
    /// 需要执行工具调用，然后继续下一轮
    ///
    /// 这是唯一需要继续循环的情况。
    ContinueWithTools {
        /// 待执行的工具调用列表
        tool_calls: Vec<LLMToolCall>,
    },

    /// 任务完成：LLM 给出了响应内容
    ///
    /// **关键洞察：只要有任何内容（thinking 或 output），就是完成。**
    /// 不需要判断内容是否"足够"或"有意义"。
    Complete {
        /// thinking 标签内的内容
        ///
        /// 即使只有 thinking 没有 output，也算完成。
        thinking: Option<String>,

        /// 可见输出内容（标签外的文本）
        output: Option<String>,
    },

    /// 真正的空响应（异常情况）
    ///
    /// LLM 既没有工具调用，也没有任何文本输出。
    /// 这通常表示：
    /// - LLM 出错
    /// - 网络问题
    /// - 其他异常情况
    ///
    /// 应该递增空闲计数器，连续多次后触发安全网终止。
    Empty,
}

impl IterationOutcome {
    /// 是否需要继续迭代
    ///
    /// 只有 ContinueWithTools 返回 true，其他都应该终止。
    pub fn should_continue(&self) -> bool {
        matches!(self, Self::ContinueWithTools { .. })
    }

    /// 是否是正常完成
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    /// 是否是空响应
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// 获取工具调用列表（如果有）
    pub fn get_tool_calls(&self) -> Option<&[LLMToolCall]> {
        match self {
            Self::ContinueWithTools { tool_calls } => Some(tool_calls),
            _ => None,
        }
    }

    /// 获取输出内容用于持久化
    ///
    /// 优先返回 output，如果没有则返回 thinking。
    pub fn get_output_for_persistence(&self) -> Option<String> {
        match self {
            Self::Complete { thinking, output } => output.clone().or_else(|| thinking.clone()),
            _ => None,
        }
    }

    /// 获取完整内容（thinking + output）
    pub fn get_full_content(&self) -> Option<(Option<String>, Option<String>)> {
        match self {
            Self::Complete { thinking, output } => Some((thinking.clone(), output.clone())),
            _ => None,
        }
    }

    /// 判断是否有实质性内容
    ///
    /// 任何 thinking 或 output 都算有内容，不需要判断长度。
    pub fn has_content(&self) -> bool {
        match self {
            Self::Complete { thinking, output } => thinking.is_some() || output.is_some(),
            Self::ContinueWithTools { tool_calls } => !tool_calls.is_empty(),
            Self::Empty => false,
        }
    }

    /// 获取人类可读的描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::ContinueWithTools { tool_calls } => {
                if tool_calls.len() == 1 {
                    "需要执行 1 个工具"
                } else {
                    "需要执行多个工具"
                }
            }
            Self::Complete { thinking, output } => match (thinking.is_some(), output.is_some()) {
                (true, true) => "完成（有思考和输出）",
                (true, false) => "完成（仅有思考）",
                (false, true) => "完成（仅有输出）",
                (false, false) => "完成（无内容）",
            },
            Self::Empty => "空响应",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_continue() {
        let continue_outcome = IterationOutcome::ContinueWithTools { tool_calls: vec![] };
        assert!(continue_outcome.should_continue());

        let complete_outcome = IterationOutcome::Complete {
            thinking: Some("思考".to_string()),
            output: None,
        };
        assert!(!complete_outcome.should_continue());

        let empty_outcome = IterationOutcome::Empty;
        assert!(!empty_outcome.should_continue());
    }

    #[test]
    fn test_has_content() {
        // 有 thinking
        let outcome1 = IterationOutcome::Complete {
            thinking: Some("思考内容".to_string()),
            output: None,
        };
        assert!(outcome1.has_content());

        // 有 output
        let outcome2 = IterationOutcome::Complete {
            thinking: None,
            output: Some("输出内容".to_string()),
        };
        assert!(outcome2.has_content());

        // 两者都有
        let outcome3 = IterationOutcome::Complete {
            thinking: Some("思考".to_string()),
            output: Some("输出".to_string()),
        };
        assert!(outcome3.has_content());

        // 都没有（异常情况，但仍然是 Complete）
        let outcome4 = IterationOutcome::Complete {
            thinking: None,
            output: None,
        };
        assert!(!outcome4.has_content());

        // Empty
        let outcome5 = IterationOutcome::Empty;
        assert!(!outcome5.has_content());
    }

    #[test]
    fn test_get_output_for_persistence() {
        // 优先 output
        let outcome1 = IterationOutcome::Complete {
            thinking: Some("思考".to_string()),
            output: Some("输出".to_string()),
        };
        assert_eq!(
            outcome1.get_output_for_persistence(),
            Some("输出".to_string())
        );

        // 如果没有 output，返回 thinking
        let outcome2 = IterationOutcome::Complete {
            thinking: Some("思考".to_string()),
            output: None,
        };
        assert_eq!(
            outcome2.get_output_for_persistence(),
            Some("思考".to_string())
        );

        // 都没有
        let outcome3 = IterationOutcome::Complete {
            thinking: None,
            output: None,
        };
        assert_eq!(outcome3.get_output_for_persistence(), None);
    }
}
