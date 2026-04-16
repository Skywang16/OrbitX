use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::warn;

use super::file_utils::lenient;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    BackoffStrategy, RateLimitConfig, RunnableTool, ToolCategory, ToolMetadata, ToolPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};

static EXA_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .pool_max_idle_per_host(2)
        .build()
        .expect("failed to build Exa HTTP client")
});

#[derive(Debug, Deserialize)]
struct WebSearchArgs {
    query: String,
    #[serde(
        alias = "numResults",
        default,
        deserialize_with = "lenient::deserialize_opt_u8"
    )]
    num_results: Option<u8>,
    #[serde(rename = "type")]
    search_type: Option<String>,
}

pub struct WebSearchTool;
impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}
impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        r#"Search the web for real-time information. Returns top results with content snippets.

Usage:
- Provide `query` keywords. Optional: `numResults` (1–10, default 5), `type` ("auto", "keyword", "neural").
- Returns a markdown list of results with title, URL, and content snippet.
- To read a page, call `web_fetch` with the URL and a specific `prompt` (question) about what you need.
- Do not rely on snippets alone. Always use web_fetch to extract detailed information from relevant pages.

Search Tips:
- Include version numbers: "React 18 useEffect cleanup"
- Search exact error messages in quotes
- Try multiple phrasings if first search fails
- Add "official" or "documentation" for authoritative results

Search Types:
- "auto": Balanced search (default)
- "keyword": Keyword-oriented retrieval
- "neural": Semantic retrieval

Examples:
- {"query": "rust async channels", "numResults": 5}
- {"query": "vue 3 composition api", "type": "keyword"}
- {"query": "machine learning tutorial", "type": "neural", "numResults": 8}
"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search keywords"},
                "numResults": {"type": "integer", "minimum": 1, "maximum": 10, "description": "Number of results (default 5)"},
                "type": {"type": "string", "enum": ["auto", "keyword", "neural"], "description": "Search type: auto (balanced), keyword (keyword-focused), neural (semantic)"}
            },
            "required": ["query"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network, ToolPriority::Standard)
            .with_rate_limit(RateLimitConfig {
                max_calls: 10,
                window_secs: 60,
                backoff: BackoffStrategy::Exponential {
                    base_ms: 800,
                    max_ms: 20_000,
                },
            })
            .with_timeout(Duration::from_secs(30))
            .with_tags(vec!["network".into(), "search".into()])
            .with_summary_key_arg("query")
    }

    async fn run(
        &self,
        _context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
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
        if !matches!(search_type, "auto" | "keyword" | "neural") {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(format!(
                    "Invalid searchType '{search_type}'. Expected one of: auto, keyword, neural"
                ))],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        let started = std::time::Instant::now();
        let res = exa_mcp_search(&args.query, num_results, search_type).await;

        match res {
            Ok(content) => {
                let results = parse_exa_results(&content);
                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(content)],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    ext_info: Some(json!({
                        "provider": "exa",
                        "type": search_type,
                        "results": results,
                        "totalFound": results.len(),
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSearchResultEntry {
    title: String,
    url: String,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_date: Option<String>,
}

const SNIPPET_MAX_LEN: usize = 300;

/// Parse Exa MCP text response into structured results.
/// Exa returns blocks separated by blank lines with "Title:", "URL:", "Score:", "Text:" fields.
fn parse_exa_results(text: &str) -> Vec<WebSearchResultEntry> {
    let mut results = Vec::new();
    let mut title = String::new();
    let mut url = String::new();
    let mut score: Option<f64> = None;
    let mut published_date: Option<String> = None;
    let mut in_text = false;
    let mut text_buf = String::new();

    let flush = |results: &mut Vec<WebSearchResultEntry>,
                 title: &mut String,
                 url: &mut String,
                 score: &mut Option<f64>,
                 published_date: &mut Option<String>,
                 text_buf: &mut String| {
        if !title.is_empty() || !url.is_empty() {
            let mut snippet = text_buf.trim().to_string();
            if snippet.len() > SNIPPET_MAX_LEN {
                snippet.truncate(SNIPPET_MAX_LEN);
                snippet.push_str("…");
            }
            results.push(WebSearchResultEntry {
                title: std::mem::take(title),
                url: std::mem::take(url),
                snippet,
                score: score.take(),
                published_date: published_date.take(),
            });
        }
        text_buf.clear();
    };

    for line in text.lines() {
        if let Some(val) = line.strip_prefix("Title: ") {
            if in_text {
                in_text = false;
            }
            flush(
                &mut results,
                &mut title,
                &mut url,
                &mut score,
                &mut published_date,
                &mut text_buf,
            );
            title = val.to_string();
        } else if let Some(val) = line.strip_prefix("URL: ") {
            in_text = false;
            url = val.to_string();
        } else if let Some(val) = line.strip_prefix("Score: ") {
            in_text = false;
            score = val.parse().ok();
        } else if let Some(val) = line.strip_prefix("Published Date: ") {
            in_text = false;
            published_date = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("Text: ") {
            in_text = true;
            text_buf.push_str(val);
        } else if line.starts_with("ID: ") || line.starts_with("Author: ") {
            in_text = false;
            // skip
        } else if in_text {
            text_buf.push('\n');
            text_buf.push_str(line);
        }
    }

    flush(
        &mut results,
        &mut title,
        &mut url,
        &mut score,
        &mut published_date,
        &mut text_buf,
    );

    results
}

const EXA_MCP_URL: &str = "https://mcp.exa.ai/mcp";

async fn exa_mcp_search(
    query: &str,
    num_results: u8,
    search_type: &str,
) -> ToolExecutorResult<String> {
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

    let response = EXA_CLIENT
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
        let error_text = response.text().await.unwrap_or_else(|err| {
            warn!(
                "Failed to read web_search error response body for status {}: {}",
                status, err
            );
            format!("<failed to read error response body: {err}>")
        });
        return Err(ToolExecutorError::ExecutionFailed {
            tool_name: "web_search".into(),
            error: format!("HTTP {status}: {error_text}"),
        });
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| ToolExecutorError::ExecutionFailed {
            tool_name: "web_search".into(),
            error: format!("Failed to read response: {e}"),
        })?;

    let mut saw_response_payload = false;

    // Parse SSE response - look for "data: " lines
    for line in response_text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            saw_response_payload = true;
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
    match serde_json::from_str::<McpResponse>(&response_text) {
        Ok(mcp_resp) => {
            saw_response_payload = true;
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
        Err(err) => {
            tracing::debug!(error = %err, "Failed to parse full Exa MCP response as JSON");
        }
    }

    if saw_response_payload {
        return Err(ToolExecutorError::ExecutionFailed {
            tool_name: "web_search".into(),
            error: "Search provider returned no usable result content".to_string(),
        });
    }

    Err(ToolExecutorError::ExecutionFailed {
        tool_name: "web_search".into(),
        error: "Search provider returned no results".to_string(),
    })
}
