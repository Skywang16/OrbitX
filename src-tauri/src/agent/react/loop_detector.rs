/*!
 * Loop Detector - 从 executor.rs 提取的循环检测逻辑
 */

use crate::agent::core::context::TaskContext;
use crate::agent::react::types::ReactIteration;

pub struct LoopDetector;

impl LoopDetector {
    pub async fn detect_loop_pattern(
        context: &TaskContext,
        current_iteration: u32,
    ) -> Option<String> {
        const LOOP_DETECTION_WINDOW: usize = 3;

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

        if let Some(warning) = Self::detect_identical_tool_sequence(&recent) {
            return Some(warning);
        }

        if let Some(warning) = Self::detect_similar_tool_pattern(&recent) {
            return Some(warning);
        }

        None
    }

    fn detect_identical_tool_sequence(recent_iterations: &[&ReactIteration]) -> Option<String> {
        if recent_iterations.len() < 2 {
            return None;
        }

        let tool_sequences: Vec<Vec<String>> = recent_iterations
            .iter()
            .map(|iter| {
                iter.action
                    .as_ref()
                    .map(|action| vec![action.tool_name.clone()])
                    .unwrap_or_default()
            })
            .collect();

        let first = &tool_sequences[0];
        if first.is_empty() {
            return None;
        }

        let all_identical = tool_sequences[1..].iter().all(|seq| seq == first);

        if all_identical {
            let tools_list = first.join(", ");
            return Some(format!(
                "<system-reminder type=\"loop-warning\">\n\
                 You've called the same tools {} times in a row: {}\n\n\
                 The results haven't changed. Consider:\n\
                 - Have you gathered enough information?\n\
                 - Can you proceed with what you have?\n\
                 - Do you need to try a different approach?\n\n\
                 Break the loop by using the information you already have or trying different tools.\n\
                 </system-reminder>",
                recent_iterations.len(),
                tools_list
            ));
        }

        None
    }

    fn detect_similar_tool_pattern(recent_iterations: &[&ReactIteration]) -> Option<String> {
        if recent_iterations.len() < 3 {
            return None;
        }

        let tool_names: Vec<Option<String>> = recent_iterations
            .iter()
            .map(|iter| iter.action.as_ref().map(|a| a.tool_name.clone()))
            .collect();

        let mut tool_counts = std::collections::HashMap::new();
        for name in tool_names.iter().flatten() {
            *tool_counts.entry(name.clone()).or_insert(0) += 1;
        }

        for (tool, count) in tool_counts {
            if count >= 3 {
                return Some(format!(
                    "<system-reminder type=\"loop-warning\">\n\
                     You've called '{}' tool {} times in the last {} iterations.\n\n\
                     You may be stuck in a pattern. Consider:\n\
                     - Are you getting new information each time?\n\
                     - Can you analyze the results you already have?\n\
                     - Should you try a different approach?\n\n\
                     Try to make progress with the information you've gathered.\n\
                     </system-reminder>",
                    tool,
                    count,
                    recent_iterations.len()
                ));
            }
        }

        None
    }
}
