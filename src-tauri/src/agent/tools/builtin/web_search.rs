use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    BackoffStrategy, RateLimitConfig, RunnableTool, ToolCategory, ToolMetadata, ToolPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};

#[derive(Debug, Deserialize)]
struct WebSearchArgs {
    /// Search query keywords
    query: String,
    /// Optional: number of results (default 5, max 10)
    #[serde(alias = "numResults")]
    num_results: Option<u8>,
    /// Optional: search type - "auto" (default), "fast", or "deep"
    #[serde(rename = "type")]
    search_type: Option<String>,
}

pub struct WebSearchTool;
impl Default for WebSearchTool {
    fn default() -> Self { Self::new() }
}
impl WebSearchTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RunnableTool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }

    fn description(&self) -> &str {
        r#"Keyword web search returning top results with content snippets. No API key required.

Usage:
- Provide `query` keywords. Optional: `numResults` (1–10, default 5), `type` ("auto", "fast", "deep").
- Returns a markdown list of results with title, URL, and content snippet.
- To read full page content, call `web_fetch` with the chosen URL.

Search Types:
- "auto": Balanced search (default)
- "fast": Quick results, less comprehensive
- "deep": Thorough search, more comprehensive

Examples:
- {"query": "rust async channels", "numResults": 5}
- {"query": "vue 3 composition api", "type": "fast"}
- {"query": "machine learning tutorial", "type": "deep", "numResults": 8}
"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search keywords"},
                "numResults": {"type": "integer", "minimum": 1, "maximum": 10, "description": "Number of results (default 5)"},
                "type": {"type": "string", "enum": ["auto", "fast", "deep"], "description": "Search type: auto (balanced), fast (quick), deep (thorough)"}
            },
            "required": ["query"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network, ToolPriority::Standard)
            .with_rate_limit(RateLimitConfig {
                max_calls: 10,
                window_secs: 60,
                backoff: BackoffStrategy::Exponential { base_ms: 800, max_ms: 20_000 },
            })
            .with_timeout(Duration::from_secs(30))
            .with_tags(vec!["network".into(), "search".into()])
            .with_summary_key_arg("query")
    }

    async fn run(&self, _context: &TaskContext, args: serde_json::Value) -> ToolExecutorResult<ToolResult> {
        let args: WebSearchArgs = serde_json::from_value(args)?;
        if args.query.trim().is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error("query is empty".into())],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        let num_results = args.num_results.unwrap_or(5).clamp(1, 10);
        let search_type = args.search_type.as_deref().unwrap_or("auto");

        let started = std::time::Instant::now();
        let res = exa_mcp_search(&args.query, num_results, search_type).await;

        match res {
            Ok(content) => {
                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(content)],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    ext_info: Some(json!({
                        "provider": "exa",
                        "type": search_type,
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                content: vec![ToolResultContent::Error(e.to_string())],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
                ext_info: None,
            }),
        }
    }
}

/// Exa MCP API request/response structures
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: &'static str,
    id: u32,
    method: &'static str,
    params: McpParams,
}

#[derive(Debug, Serialize)]
struct McpParams {
    name: &'static str,
    arguments: McpArguments,
}

#[derive(Debug, Serialize)]
struct McpArguments {
    query: String,
    #[serde(rename = "numResults")]
    num_results: u8,
    #[serde(rename = "type")]
    search_type: String,
    livecrawl: &'static str,
}

#[derive(Debug, Deserialize)]
struct McpResponse {
    result: Option<McpResult>,
    error: Option<McpError>,
}

#[derive(Debug, Deserialize)]
struct McpResult {
    content: Vec<McpContent>,
}

#[derive(Debug, Deserialize)]
struct McpContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct McpError {
    message: String,
}

const EXA_MCP_URL: &str = "https://mcp.exa.ai/mcp";

async fn exa_mcp_search(query: &str, num_results: u8, search_type: &str) -> ToolExecutorResult<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()?;

    let request = McpRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: McpParams {
            name: "web_search_exa",
            arguments: McpArguments {
                query: query.to_string(),
                num_results,
                search_type: search_type.to_string(),
                livecrawl: "fallback",
            },
        },
    };

    let response = client
        .post(EXA_MCP_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&request)
        .send()
        .await
        .map_err(|e| ToolExecutorError::ExecutionFailed {
            tool_name: "web_search".into(),
            error: format!("Request failed: {e}"),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(ToolExecutorError::ExecutionFailed {
            tool_name: "web_search".into(),
            error: format!("HTTP {}: {}", status, error_text),
        });
    }

    let response_text = response.text().await.map_err(|e| ToolExecutorError::ExecutionFailed {
        tool_name: "web_search".into(),
        error: format!("Failed to read response: {e}"),
    })?;

    // Parse SSE response - look for "data: " lines
    for line in response_text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            match serde_json::from_str::<McpResponse>(data) {
                Ok(mcp_resp) => {
                    if let Some(error) = mcp_resp.error {
                        return Err(ToolExecutorError::ExecutionFailed {
                            tool_name: "web_search".into(),
                            error: error.message,
                        });
                    }
                    if let Some(result) = mcp_resp.result {
                        if let Some(content) = result.content.first() {
                            tracing::info!(query = %query, "Exa MCP search succeeded");
                            return Ok(content.text.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "Failed to parse MCP response line");
                }
            }
        }
    }

    // If no SSE data lines, try parsing the whole response as JSON
    if let Ok(mcp_resp) = serde_json::from_str::<McpResponse>(&response_text) {
        if let Some(error) = mcp_resp.error {
            return Err(ToolExecutorError::ExecutionFailed {
                tool_name: "web_search".into(),
                error: error.message,
            });
        }
        if let Some(result) = mcp_resp.result {
            if let Some(content) = result.content.first() {
                return Ok(content.text.clone());
            }
        }
    }

    Ok("No search results found. Please try a different query.".to_string())
}
