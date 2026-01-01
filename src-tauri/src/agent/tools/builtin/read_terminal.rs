use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};
use crate::completion::output_analyzer::OutputAnalyzer;
use crate::mux::singleton::get_mux;
use crate::mux::PaneId;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadTerminalArgs {
    max_lines: Option<usize>,
}

pub struct ReadTerminalTool;

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
        "Reads the current visible content from the active terminal pane.

Usage:
- Returns the terminal output buffer that the user is currently viewing
- Useful for analyzing terminal errors, command outputs, or debugging issues
- This is NOT for general code editing - use read_file for reading source files
- Use this when the user asks you to analyze what they see in the terminal
- The content includes ANSI escape codes and terminal formatting"
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

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Terminal]
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

        // 从OutputAnalyzer获取终端缓冲区内容
        let buffer = match OutputAnalyzer::global().get_pane_buffer(pane_id.as_u32()) {
            Ok(content) => content,
            Err(err) => {
                return Ok(tool_error(format!(
                    "Failed to read terminal buffer: {}",
                    err
                )));
            }
        };

        if buffer.is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(
                    "Terminal buffer is empty.".to_string(),
                )],
                is_error: false,
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
        let start_index = if total_lines > max_lines {
            total_lines - max_lines
        } else {
            0
        };

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
            is_error: false,
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

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}
