use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::mcp::client::McpClient;
use crate::agent::mcp::types::McpToolDefinition;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool_def: McpToolDefinition,
    qualified_name: String,
}

impl McpToolAdapter {
    pub fn new(client: Arc<McpClient>, tool_def: McpToolDefinition) -> Self {
        let qualified_name = format!("mcp__{}__{}", client.name(), tool_def.name);
        Self {
            client,
            tool_def,
            qualified_name,
        }
    }
}

#[async_trait]
impl RunnableTool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.qualified_name
    }

    fn description(&self) -> &str {
        self.tool_def.description.as_str()
    }

    fn parameters_schema(&self) -> Value {
        self.tool_def.input_schema.clone()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Execution, ToolPriority::Standard)
            .with_tags(vec!["mcp".into(), self.client.name().into()])
    }

    async fn run(&self, _ctx: &TaskContext, args: Value) -> ToolExecutorResult<ToolResult> {
        let res = self.client.call_tool(&self.tool_def.name, args).await;
        match res {
            Ok(call) => {
                if call.is_error {
                    Ok(ToolResult {
                        content: vec![ToolResultContent::Error(format!(
                            "mcp tool error: {}",
                            serde_json::to_string(&call.content).unwrap_or_default()
                        ))],
                        status: ToolResultStatus::Error,
                        cancel_reason: None,
                        execution_time_ms: None,
                        ext_info: Some(serde_json::json!({ "content": call.content })),
                    })
                } else {
                    Ok(ToolResult {
                        content: vec![ToolResultContent::Success(format!(
                            "mcp tool ok: {}",
                            serde_json::to_string(&call.content).unwrap_or_default()
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: None,
                        ext_info: Some(serde_json::json!({ "content": call.content })),
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                content: vec![ToolResultContent::Error(format!("mcp call failed: {e}"))],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            }),
        }
    }
}
