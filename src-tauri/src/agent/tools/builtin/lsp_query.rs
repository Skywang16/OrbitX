use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::lsp::LspManager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LspQueryArgs {
    action: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    character: Option<u32>,
}

pub struct LspQueryTool {
    manager: Arc<LspManager>,
}

impl LspQueryTool {
    pub fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl RunnableTool for LspQueryTool {
    fn name(&self) -> &str {
        "lsp_query"
    }

    fn description(&self) -> &str {
        "Query language servers for document symbols, workspace symbols, hover, definitions, references, diagnostics, or status. Use this when tree-sitter/search is not enough and you need IDE-grade semantic results."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "document_symbols", "workspace_symbols", "hover", "definition", "references", "diagnostics"]
                },
                "path": { "type": "string", "description": "Absolute or workspace-relative file path when the action targets a file." },
                "query": { "type": "string", "description": "Required for workspace_symbols." },
                "line": { "type": "integer", "description": "0-based line for hover/definition/references." },
                "character": { "type": "integer", "description": "0-based UTF-16 character for hover/definition/references." }
            },
            "required": ["action"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Expensive)
            .with_tags(vec!["lsp".into(), "semantic".into()])
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: LspQueryArgs = serde_json::from_value(args)?;
        let workspace = context.cwd.as_ref();

        let result = match args.action.as_str() {
            "status" => serde_json::to_value(self.manager.status().await)?,
            "document_symbols" => {
                let path = require_path(&args)?;
                serde_json::to_value(
                    self.manager
                        .document_symbols(workspace, &path)
                        .await
                        .map_err(map_lsp_error)?,
                )?
            }
            "workspace_symbols" => serde_json::to_value(
                self.manager
                    .workspace_symbols(workspace, args.path.as_deref(), require_query(&args)?)
                    .await
                    .map_err(map_lsp_error)?,
            )?,
            "hover" => {
                let path = require_path(&args)?;
                let (line, character) = require_position(&args)?;
                serde_json::to_value(
                    self.manager
                        .hover(workspace, &path, line, character)
                        .await
                        .map_err(map_lsp_error)?,
                )?
            }
            "definition" => {
                let path = require_path(&args)?;
                let (line, character) = require_position(&args)?;
                serde_json::to_value(
                    self.manager
                        .definition(workspace, &path, line, character)
                        .await
                        .map_err(map_lsp_error)?,
                )?
            }
            "references" => {
                let path = require_path(&args)?;
                let (line, character) = require_position(&args)?;
                serde_json::to_value(
                    self.manager
                        .references(workspace, &path, line, character)
                        .await
                        .map_err(map_lsp_error)?,
                )?
            }
            "diagnostics" => serde_json::to_value(
                self.manager
                    .diagnostics(workspace, args.path.as_deref())
                    .await
                    .map_err(map_lsp_error)?,
            )?,
            other => {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "lsp_query".to_string(),
                    error: format!("unsupported action: {other}"),
                });
            }
        };

        let pretty = serde_json::to_string_pretty(&result).map_err(|err| {
            ToolExecutorError::ExecutionFailed {
                tool_name: "lsp_query".to_string(),
                error: format!("failed to serialize LSP result: {err}"),
            }
        })?;
        Ok(ToolResult {
            content: vec![ToolResultContent::Success(pretty)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(result),
        })
    }
}

fn require_path(args: &LspQueryArgs) -> ToolExecutorResult<String> {
    args.path.clone().ok_or_else(
        || crate::agent::error::ToolExecutorError::InvalidArguments {
            tool_name: "lsp_query".to_string(),
            error: "path is required for this action".to_string(),
        },
    )
}

fn require_query(args: &LspQueryArgs) -> ToolExecutorResult<String> {
    let query = args
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(
            || crate::agent::error::ToolExecutorError::InvalidArguments {
                tool_name: "lsp_query".to_string(),
                error: "query is required for workspace_symbols".to_string(),
            },
        )?;
    Ok(query.to_string())
}

fn require_position(args: &LspQueryArgs) -> ToolExecutorResult<(u32, u32)> {
    let line =
        args.line.ok_or_else(
            || crate::agent::error::ToolExecutorError::InvalidArguments {
                tool_name: "lsp_query".to_string(),
                error: "line is required for this action".to_string(),
            },
        )?;
    let character =
        args.character.ok_or_else(
            || crate::agent::error::ToolExecutorError::InvalidArguments {
                tool_name: "lsp_query".to_string(),
                error: "character is required for this action".to_string(),
            },
        )?;
    Ok((line, character))
}

fn map_lsp_error(err: crate::lsp::manager::LspManagerError) -> ToolExecutorError {
    ToolExecutorError::ExecutionFailed {
        tool_name: "lsp_query".to_string(),
        error: err.to_string(),
    }
}
