/*!
 * Executor Helpers - 从 executor.rs 提取的辅助函数
 */

use crate::agent::core::context::ToolCallResult;
use crate::agent::persistence::ExecutionMessage;
use crate::agent::tools::{ToolResult, ToolResultContent, ToolResultStatus};
use crate::llm::anthropic_types::{MessageContent, MessageParam};

/// 去重工具调用 - 检测同一iteration内的重复调用
pub fn deduplicate_tool_uses(
    tool_calls: &[(String, String, serde_json::Value)],
) -> Vec<(String, String, serde_json::Value)> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut deduplicated = Vec::new();

    for (id, name, args) in tool_calls.iter() {
        let key = (
            name.clone(),
            serde_json::to_string(args).unwrap_or_default(),
        );

        if seen.insert(key) {
            deduplicated.push((id.clone(), name.clone(), args.clone()));
        }
    }

    deduplicated
}

/// 将 ToolCallResult 转换为 ToolOutcome
pub fn tool_call_result_to_outcome(result: &ToolCallResult) -> ToolResult {
    let content = match result.status {
        ToolResultStatus::Success => {
            let result_str = serde_json::to_string(&result.result)
                .unwrap_or_else(|_| "Tool execution succeeded".to_string());
            ToolResultContent::Success(result_str)
        }
        ToolResultStatus::Error => {
            let message = result
                .result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Tool execution failed")
                .to_string();
            ToolResultContent::Error(message)
        }
        ToolResultStatus::Cancelled => {
            let message = result
                .result
                .get("cancelled")
                .and_then(|v| v.as_str())
                .unwrap_or("Tool execution cancelled")
                .to_string();
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

/// 从Vec尾部获取指定数量的元素
pub fn tail_vec<T: Clone>(items: Vec<T>, limit: usize) -> Vec<T> {
    if limit == 0 || items.len() <= limit {
        items
    } else {
        items[items.len() - limit..].to_vec()
    }
}

/// 将数据库存储的ExecutionMessage转换为Anthropic原生MessageParam
pub fn convert_execution_messages(messages: &[ExecutionMessage]) -> Vec<MessageParam> {
    messages
        .iter()
        .map(|msg| MessageParam {
            role: match msg.role.as_str() {
                "user" => crate::llm::anthropic_types::MessageRole::User,
                _ => crate::llm::anthropic_types::MessageRole::Assistant,
            },
            content: MessageContent::Text(msg.content.clone()),
        })
        .collect()
}
