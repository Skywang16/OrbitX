use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::mux::singleton::get_mux;
use crate::mux::PaneId;
use crate::terminal::TerminalScrollback;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadTerminalArgs {
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
        r#"Reads the current visible content from the active terminal pane.

Usage:
- Returns the terminal output buffer that the user is currently viewing
- Includes recent command outputs, error messages, and terminal history
- Useful for analyzing terminal errors or debugging command failures
- Use maxLines parameter to control how much history to retrieve (default: 1000)

Note: This is NOT for reading source files - use read_file instead."#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "maxLines": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 10000,
                    "description": "Maximum number of lines to return from the terminal buffer. Default: 1000. Use lower values for recent output, higher values for full history."
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
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadTerminalArgs = serde_json::from_value(args)?;
        let max_lines = args.max_lines.unwrap_or(1000);

        // 获取活跃终端的pane_id
        // 优先使用 mux 中的第一个可用 pane
        let mux = get_mux();
        let pane_id = mux.list_panes().into_iter().next().ok_or_else(|| {
            crate::agent::error::ToolExecutorError::ExecutionFailed {
                tool_name: "read_terminal".to_string(),
                error: "No terminal panes found. Please ensure a terminal is open.".to_string(),
            }
        })?;

        let buffer = TerminalScrollback::global().get_text_lossy(pane_id.as_u32());

        if buffer.is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(
                    "Terminal buffer is empty.".to_string(),
                )],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "paneId": pane_id.as_u32(),
                    "lineCount": 0,
                    "isEmpty": true
                })),
            });
        }

        // 按行分割并限制行数
        let lines: Vec<&str> = buffer.lines().collect();
        let total_lines = lines.len();
        let lines_to_return = total_lines.min(max_lines);

        // 取最后N行（最新的内容）
        let start_index = total_lines.saturating_sub(max_lines);

        let selected_lines: Vec<&str> = lines.iter().skip(start_index).copied().collect();
        let result_text = selected_lines.join("\n");

        // 获取终端大小信息
        let mux = get_mux();
        let size = mux
            .get_pane(PaneId::new(pane_id.as_u32()))
            .map(|pane| pane.get_size())
            .unwrap_or_default();

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(result_text)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "paneId": pane_id.as_u32(),
                "totalLines": total_lines,
                "returnedLines": lines_to_return,
                "truncated": total_lines > max_lines,
                "terminalSize": {
                    "cols": size.cols,
                    "rows": size.rows
                }
            })),
        })
    }
}
