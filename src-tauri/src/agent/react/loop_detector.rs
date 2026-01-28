//! Loop Detector - 检测 agent 陷入循环的情况
//!
//! 检测真正的重复：相同工具 + 相同参数，而不是仅靠工具名。
//! 读取不同文件的多次 read_file 调用是正常行为，不应触发警告。

use std::collections::HashMap;

use serde_json::Value;

use crate::agent::core::context::TaskContext;
use crate::agent::prompt::PromptBuilder;
use crate::agent::react::types::ReactIteration;

/// 工具调用签名：(工具名, 参数)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ToolSignature {
    name: String,
    /// 参数的规范化字符串（用于比较）
    args_hash: String,
}

impl ToolSignature {
    fn from_action(name: &str, args: &Value) -> Self {
        Self {
            name: name.to_string(),
            args_hash: Self::normalize_args(args),
        }
    }

    /// 规范化参数为可比较的字符串
    fn normalize_args(args: &Value) -> String {
        // 对于 JSON 对象，按 key 排序后序列化，确保相同参数产生相同字符串
        match args {
            Value::Object(map) => {
                let mut pairs: Vec<_> = map.iter().collect();
                pairs.sort_by_key(|(k, _)| *k);
                let sorted: serde_json::Map<String, Value> =
                    pairs.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                serde_json::to_string(&Value::Object(sorted)).unwrap_or_default()
            }
            _ => serde_json::to_string(args).unwrap_or_default(),
        }
    }
}

pub struct LoopDetector;

impl LoopDetector {
    /// 检测循环模式
    ///
    /// 只有当检测到**完全相同的工具调用**（相同工具名 + 相同参数）重复时才触发警告。
    /// 调用不同参数的同一工具（如读取不同文件）不会触发警告。
    pub async fn detect_loop_pattern(
        context: &TaskContext,
        current_iteration: u32,
    ) -> Option<String> {
        const LOOP_DETECTION_WINDOW: usize = 3;
        const MIN_IDENTICAL_CALLS: usize = 2;

        if current_iteration < LOOP_DETECTION_WINDOW as u32 {
            return None;
        }

        let react = context.states.react_runtime.read().await;
        let snapshot = react.get_snapshot();
        let iterations = &snapshot.iterations;

        if iterations.len() < LOOP_DETECTION_WINDOW {
            return None;
        }

        let recent: Vec<_> = iterations
            .iter()
            .rev()
            .take(LOOP_DETECTION_WINDOW)
            .collect();

        // 检测完全相同的工具调用（相同工具 + 相同参数）
        Self::detect_identical_tool_calls(&recent, MIN_IDENTICAL_CALLS)
    }

    /// 检测完全相同的工具调用
    ///
    /// 只有当同一个工具以**完全相同的参数**被调用多次时才触发警告。
    fn detect_identical_tool_calls(
        recent_iterations: &[&ReactIteration],
        min_count: usize,
    ) -> Option<String> {
        // 收集所有工具调用签名
        let mut call_counts: HashMap<ToolSignature, usize> = HashMap::new();

        for iter in recent_iterations {
            if let Some(action) = &iter.action {
                let sig = ToolSignature::from_action(&action.tool_name, &action.arguments);
                *call_counts.entry(sig).or_insert(0) += 1;
            }
        }

        // 找出重复调用次数最多的
        let mut duplicates: Vec<_> = call_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_count)
            .collect();

        if duplicates.is_empty() {
            return None;
        }

        // 按重复次数排序，取最严重的
        duplicates.sort_by(|a, b| b.1.cmp(&a.1));
        let (sig, count) = &duplicates[0];

        let builder = PromptBuilder::new(None);
        let warning = builder.get_loop_warning(*count, &sig.name);
        Some(format!(
            "<system-reminder type=\"loop-warning\">\n{}\n</system-reminder>",
            warning
        ))
    }
}
