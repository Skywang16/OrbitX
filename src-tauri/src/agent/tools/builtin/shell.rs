use async_trait::async_trait;
use serde_json::json;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    RunnableTool, ToolExecutorResult, ToolPermission, ToolResult, ToolResultContent,
};

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands in a controlled environment with safety checks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute"
                },
                "paneId": {
                    "type": "number",
                    "description": "Optional terminal pane identifier"
                }
            },
            "required": ["command"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::SystemCommand]
    }

    fn tags(&self) -> Vec<String> {
        vec!["shell".into(), "command".into(), "system".into()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        _args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        Ok(migration_pending_result(
            self.name(),
            "Backend shell implementation pending migration from front-end",
        ))
    }
}

fn migration_pending_result(tool_name: &str, message: &str) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Text {
            text: message.to_string(),
        }],
        is_error: true,
        execution_time_ms: None,
        metadata: Some(json!({
            "tool": tool_name,
            "status": "pending_migration"
        })),
    }
}
