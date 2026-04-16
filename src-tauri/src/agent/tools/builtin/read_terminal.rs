use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::file_utils::lenient;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::terminal::AgentTerminalManager;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::mux::singleton::get_mux;
use crate::terminal::TerminalScrollback;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadTerminalArgs {
    terminal_id: Option<String>,
    #[serde(default, deserialize_with = "lenient::deserialize_opt_usize")]
    max_lines: Option<usize>,
}

pub struct ReadTerminalTool;

impl Default for ReadTerminalTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadTerminalTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ReadTerminalTool {
    fn name(&self) -> &str {
        "read_terminal"
    }

    fn description(&self) -> &str {
        r#"Reads terminal output from the active terminal or a task terminal.

Usage:
- Without terminalId: reads the user's active terminal pane
- With terminalId: reads a task terminal created by shell, especially background runs
- Use maxLines to control how much history to retrieve (default: 1000)

Note: This is NOT for reading source files - use read_file instead."#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "terminalId": {
                    "type": "string",
                    "description": "Task terminal ID returned by shell. If omitted, reads the active terminal pane."
                },
                "maxLines": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 10000,
                    "description": "Maximum number of lines to return. Default: 1000."
                }
            }
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Terminal, ToolPriority::Standard)
            .with_tags(vec!["terminal".into(), "debug".into()])
    }

    async fn run(
        &self,
        _context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadTerminalArgs = serde_json::from_value(args)?;
        let max_lines = args.max_lines.unwrap_or(1000);
        if max_lines == 0 {
            return Err(ToolExecutorError::InvalidArguments {
                tool_name: "read_terminal".to_string(),
                error: "maxLines must be greater than 0".to_string(),
            });
        }

        let (buffer, pane_id) = if let Some(ref tid) = args.terminal_id {
            read_agent_terminal(tid)?
        } else {
            read_active_terminal()?
        };

        if buffer.is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(
                    "Terminal buffer is empty.".to_string(),
                )],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "paneId": pane_id,
                    "lineCount": 0,
                    "isEmpty": true
                })),
            });
        }

        let lines: Vec<&str> = buffer.lines().collect();
        let total_lines = lines.len();
        let start_index = total_lines.saturating_sub(max_lines);
        let selected_lines: Vec<&str> = lines.iter().skip(start_index).copied().collect();
        let result_text = selected_lines.join("\n");

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(result_text)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "paneId": pane_id,
                "terminalId": args.terminal_id,
                "totalLines": total_lines,
                "returnedLines": total_lines.min(max_lines),
                "truncated": total_lines > max_lines,
            })),
        })
    }
}

fn read_active_terminal() -> ToolExecutorResult<(String, u32)> {
    let mux = get_mux();
    let pane_id =
        mux.list_panes()
            .into_iter()
            .next()
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "read_terminal".to_string(),
                error: "No terminal panes found. Please ensure a terminal is open.".to_string(),
            })?;
    let buffer = TerminalScrollback::global().get_text_lossy(pane_id.as_u32());
    Ok((buffer, pane_id.as_u32()))
}

fn read_agent_terminal(terminal_id: &str) -> ToolExecutorResult<(String, u32)> {
    let manager =
        AgentTerminalManager::global().ok_or_else(|| ToolExecutorError::ExecutionFailed {
            tool_name: "read_terminal".to_string(),
            error: "Agent terminal manager is not initialized.".to_string(),
        })?;
    let terminal =
        manager
            .get_terminal(terminal_id)
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "read_terminal".to_string(),
                error: format!("Terminal '{terminal_id}' not found."),
            })?;
    let buffer = manager.get_terminal_output(terminal_id).map_err(|e| {
        ToolExecutorError::ExecutionFailed {
            tool_name: "read_terminal".to_string(),
            error: e,
        }
    })?;
    Ok((buffer, terminal.pane_id))
}
