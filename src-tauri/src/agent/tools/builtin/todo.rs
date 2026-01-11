//! TodoWrite tool - outputs todo list to chat history only
//!
//! Simple task tracking - just pending/completed states.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority as MetaPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};

const DESCRIPTION: &str = r#"Track task progress with a simple todo list.

Rules:
1) Submit the FULL list every time (replaces previous)
2) Keep IDs stable across updates
3) At most ONE item can be in_progress at a time

Status values: pending, in_progress, completed"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoWriteArgs {
    todos: Vec<TodoItemInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoItemInput {
    content: String,
    status: String,
}

pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for TodoWriteTool {
    fn name(&self) -> &str {
        "todowrite"
    }

    fn description(&self) -> &str {
        DESCRIPTION
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, MetaPriority::Standard)
            .with_tags(vec!["todo".into(), "planning".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: TodoWriteArgs = serde_json::from_value(args)?;

        // Validate: at most one in_progress
        let in_progress_count = args
            .todos
            .iter()
            .filter(|t| t.status == "in_progress")
            .count();

        if in_progress_count > 1 {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(
                    "At most one todo can be in_progress".to_string(),
                )],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        // Format output
        let done = args
            .todos
            .iter()
            .filter(|t| t.status == "completed")
            .count();
        let total = args.todos.len();

        let mut output = format!("Todo ({}/{})\n", done, total);
        for t in &args.todos {
            let icon = match t.status.as_str() {
                "in_progress" => "▶",
                "completed" => "✓",
                _ => "○",
            };
            output.push_str(&format!("{} {}\n", icon, t.content));
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(output)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({ "done": done, "total": total })),
        })
    }
}
