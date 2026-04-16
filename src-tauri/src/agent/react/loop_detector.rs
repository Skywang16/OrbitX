//! Loop Detector - detects when agent gets stuck in loops
//!
//! Detects true duplicates: same tool + same parameters, not just tool name.
//! Multiple read_file calls reading different files are normal behavior and should not trigger warnings.

use std::collections::HashMap;

use serde_json::Value;

use crate::agent::core::context::AgentRunContext;
use crate::agent::prompt::PromptBuilder;
use crate::agent::react::types::ReactIteration;

/// Tool call signature: (tool name, parameters)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ToolSignature {
    name: String,
    /// Normalized string of parameters (for comparison)
    args_hash: String,
}

impl ToolSignature {
    fn from_action(name: &str, args: &Value) -> Self {
        Self {
            name: name.to_string(),
            args_hash: Self::normalize_args(args),
        }
    }

    /// Normalize parameters to a comparable string
    fn normalize_args(args: &Value) -> String {
        // For JSON objects, sort by key then serialize to ensure same parameters produce same string
        match args {
            Value::Object(map) => {
                let mut pairs: Vec<_> = map.iter().collect();
                pairs.sort_by_key(|(k, _)| *k);
                let sorted: serde_json::Map<String, Value> = pairs
                    .into_iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Value::Object(sorted).to_string()
            }
            _ => args.to_string(),
        }
    }
}

pub struct LoopDetector;

impl LoopDetector {
    /// Detect loop pattern - identical tool calls with same parameters
    pub async fn detect_loop_pattern(
        context: &AgentRunContext,
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

        Self::detect_identical_tool_calls(&recent, MIN_IDENTICAL_CALLS)
    }

    /// Detect identical tool calls
    fn detect_identical_tool_calls(
        recent_iterations: &[&ReactIteration],
        min_count: usize,
    ) -> Option<String> {
        let mut call_counts: HashMap<ToolSignature, usize> = HashMap::new();

        for iter in recent_iterations {
            if let Some(action) = &iter.action {
                let sig = ToolSignature::from_action(&action.tool_name, &action.arguments);
                *call_counts.entry(sig).or_insert(0) += 1;
            }
        }

        let mut duplicates: Vec<_> = call_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_count)
            .collect();

        if duplicates.is_empty() {
            return None;
        }

        duplicates.sort_by(|a, b| b.1.cmp(&a.1));
        let (sig, count) = &duplicates[0];

        let builder = PromptBuilder::new(None);
        let warning = builder.get_loop_warning(*count, &sig.name);
        Some(format!(
            "<system-reminder type=\"loop-warning\">\n{warning}\n</system-reminder>"
        ))
    }
}
