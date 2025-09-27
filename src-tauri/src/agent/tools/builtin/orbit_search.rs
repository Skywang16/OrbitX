use async_trait::async_trait;
use serde_json::json;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    RunnableTool, ToolExecutorResult, ToolPermission, ToolResult, ToolResultContent,
};

pub struct OrbitSearchTool;

impl OrbitSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for OrbitSearchTool {
    fn name(&self) -> &str {
        "orbit_search"
    }

    fn description(&self) -> &str {
        "Search for relevant code snippets using semantic or hybrid matching modes."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language description of the target code"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Maximum number of results to return"
                },
                "path": {
                    "type": "string",
                    "description": "Optional path to restrict the search scope"
                },
                "mode": {
                    "type": "string",
                    "enum": ["semantic", "hybrid", "regex"],
                    "description": "Search mode"
                }
            },
            "required": ["query"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["search".into(), "code".into(), "semantic".into()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        _args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        Ok(migration_pending_result(
            self.name(),
            "Backend orbit_search implementation pending migration from front-end",
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
