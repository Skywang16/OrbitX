/*!
 * Executor Helpers - helper functions extracted from executor.rs
 */

use crate::agent::core::context::{AgentRunContext, AgentToolCallResult};
use crate::agent::tools::{ToolResult, ToolResultContent, ToolResultStatus};

/// Deduplicate tool calls - detect duplicate calls within the same iteration
pub fn deduplicate_tool_uses(
    tool_calls: &[(String, String, serde_json::Value)],
) -> Vec<(String, String, serde_json::Value)> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut deduplicated = Vec::new();

    for (id, name, args) in tool_calls.iter() {
        let Ok(serialized_args) = serde_json::to_string(args) else {
            deduplicated.push((id.clone(), name.clone(), args.clone()));
            continue;
        };
        let key = (name.clone(), serialized_args);

        if seen.insert(key) {
            deduplicated.push((id.clone(), name.clone(), args.clone()));
        }
    }

    deduplicated
}

/// Convert AgentToolCallResult to ToolResult
pub fn tool_call_result_to_outcome(result: &AgentToolCallResult) -> ToolResult {
    let content = match result.status {
        ToolResultStatus::Success => {
            let result_str = match serde_json::to_string(&result.result) {
                Ok(serialized) => serialized,
                Err(err) => format!("<failed to serialize tool result: {err}>"),
            };
            ToolResultContent::Success(result_str)
        }
        ToolResultStatus::Error => {
            let message = match result.result.get("error").and_then(|v| v.as_str()) {
                Some(message) => message.to_string(),
                None => "Tool returned error status without error message".to_string(),
            };
            ToolResultContent::Error(message)
        }
        ToolResultStatus::Cancelled => {
            let message = match result.result.get("cancelled").and_then(|v| v.as_str()) {
                Some(message) => message.to_string(),
                None => "Tool returned cancelled status without cancel reason".to_string(),
            };
            ToolResultContent::Error(message)
        }
    };

    ToolResult {
        content: vec![content],
        status: result.status,
        cancel_reason: None,
        execution_time_ms: Some(result.execution_time_ms),
        ext_info: None,
    }
}

pub async fn should_render_tool_block(context: &AgentRunContext, tool_name: &str) -> bool {
    let Some(meta) = context.tool_registry().get_tool_metadata(tool_name).await else {
        return true;
    };
    !meta.tags.iter().any(|t| t == "ui:hidden")
}
