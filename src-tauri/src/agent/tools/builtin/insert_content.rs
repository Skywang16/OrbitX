/*!
 * Insert Content Tool
 * Insert a text snippet into a file at a given position, supports previewOnly.
 */

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolPermission, ToolResult, ToolResultContent,
};

#[derive(Debug, Deserialize)]
struct InsertContentArgs {
    path: String,
    content: String,
    #[serde(default)]
    line: Option<usize>, // 1-based
    #[serde(default)]
    after: Option<String>,
    #[serde(default)]
    before: Option<String>,
    #[serde(default, rename = "previewOnly")]
    preview_only: Option<bool>,
}

pub struct InsertContentTool;
impl InsertContentTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for InsertContentTool {
    fn name(&self) -> &str {
        "insert_content"
    }

    fn description(&self) -> &str {
        "Insert a text snippet into a file by line number, after a marker, or before a marker."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "line": { "type": "integer", "minimum": 1 },
                "after": { "type": "string" },
                "before": { "type": "string" },
                "previewOnly": { "type": "boolean", "default": false }
            },
            "oneOf": [
                { "required": ["path", "content", "line"] },
                { "required": ["path", "content", "after"] },
                { "required": ["path", "content", "before"] }
            ]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }
    fn tags(&self) -> Vec<String> {
        vec!["file".to_string(), "insert".to_string()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: InsertContentArgs = serde_json::from_value(args)?;
        let preview_only = args.preview_only.unwrap_or(false);

        // stat
        let meta = match fs::metadata(&args.path).await {
            Ok(m) => m,
            Err(e) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error {
                        message: format!("file not found: {}", e),
                        details: Some(args.path),
                    }],
                    is_error: true,
                    execution_time_ms: None,
                    metadata: None,
                });
            }
        };
        if meta.is_dir() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error {
                    message: "path is a directory".to_string(),
                    details: Some(args.path),
                }],
                is_error: true,
                execution_time_ms: None,
                metadata: None,
            });
        }

        let original = fs::read_to_string(&args.path).await?;
        let mut lines: Vec<String> = original.split('\n').map(|s| s.to_string()).collect();

        // decide insertion index
        let idx = if let Some(line_no) = args.line {
            line_no.saturating_sub(1).min(lines.len())
        } else if let Some(marker) = &args.after {
            match lines.iter().position(|l| l.contains(marker)) {
                Some(i) => (i + 1).min(lines.len()),
                None => lines.len(),
            }
        } else if let Some(marker) = &args.before {
            match lines.iter().position(|l| l.contains(marker)) {
                Some(i) => i,
                None => lines.len(),
            }
        } else {
            lines.len()
        };

        // insert maintaining line structure
        let insert_lines: Vec<String> = args.content.split('\n').map(|s| s.to_string()).collect();
        lines.splice(idx..idx, insert_lines.clone());
        let new_content = lines.join("\n");

        if !preview_only {
            fs::write(&args.path, &new_content).await?;
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Text {
                text: if preview_only {
                    format!("insert_content preview at line {}", idx + 1)
                } else {
                    format!("insert_content applied at line {}", idx + 1)
                },
            }],
            is_error: false,
            execution_time_ms: None,
            metadata: Some(json!({
                "path": args.path,
                "insertLine": idx + 1,
                "by": if args.line.is_some() { "line" } else if args.after.is_some() { "after" } else if args.before.is_some() { "before" } else { "append" },
                "previewOnly": preview_only
            })),
        })
    }
}
