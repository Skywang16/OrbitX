use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::terminal::AgentTerminalManager;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadAgentTerminalArgs {
    terminal_id: String,
    max_lines: Option<usize>,
}

pub struct ReadAgentTerminalTool;

impl Default for ReadAgentTerminalTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadAgentTerminalTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ReadAgentTerminalTool {
    fn name(&self) -> &str {
        "read_agent_terminal"
    }

    fn description(&self) -> &str {
        r#"Reads output from a specific agent terminal by terminalId.

Usage:
- terminalId: The agent terminal ID (required)
- maxLines: Max lines to return from the buffer (optional, default 1000)

Notes:
- Use this for background agent terminals
- Output is captured from the terminal buffer, may include ANSI codes
"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "terminalId": {
                    "type": "string",
                    "description": "The agent terminal ID to read output from."
                },
                "maxLines": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 10000,
                    "description": "Maximum number of lines to return from the terminal buffer. Default: 1000."
                }
            },
            "required": ["terminalId"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Terminal, ToolPriority::Standard)
            .with_tags(vec!["terminal".into(), "agent".into()])
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadAgentTerminalArgs = serde_json::from_value(args)?;
        let max_lines = args.max_lines.unwrap_or(1000);

        let manager = AgentTerminalManager::global().ok_or_else(|| {
            ToolExecutorError::ExecutionFailed {
                tool_name: "read_agent_terminal".to_string(),
                error: "Agent terminal manager is not initialized.".to_string(),
            }
        })?;

        let terminal = manager
            .get_terminal(&args.terminal_id)
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "read_agent_terminal".to_string(),
                error: "Terminal not found.".to_string(),
            })?;

        let buffer = manager
            .get_terminal_output(&args.terminal_id)
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "read_agent_terminal".to_string(),
                error: e,
            })?;

        if buffer.is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(
                    "Terminal buffer is empty.".to_string(),
                )],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "terminalId": terminal.id,
                    "paneId": terminal.pane_id,
                    "lineCount": 0,
                    "isEmpty": true
                })),
            });
        }

        let lines: Vec<&str> = buffer.lines().collect();
        let total_lines = lines.len();
        let lines_to_return = total_lines.min(max_lines);
        let start_index = total_lines.saturating_sub(max_lines);
        let selected_lines: Vec<&str> = lines.iter().skip(start_index).copied().collect();
        let result_text = selected_lines.join("\n");

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(result_text)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "terminalId": terminal.id,
                "paneId": terminal.pane_id,
                "totalLines": total_lines,
                "returnedLines": lines_to_return,
                "truncated": total_lines > max_lines,
            })),
        })
    }
}
