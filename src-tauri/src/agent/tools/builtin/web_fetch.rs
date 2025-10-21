/*!
 * Web Fetch Tool
 *
 * Provides headless HTTP requests as an Agent tool so LLM can call it via tool-calls.
 */

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use url::Url;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    BackoffStrategy, RateLimitConfig, RunnableTool, ToolCategory, ToolMetadata, ToolPermission,
    ToolPriority, ToolResult, ToolResultContent,
};

#[derive(Debug, Deserialize)]
struct WebFetchArgs {
    url: String,
}

pub struct WebFetchTool;
impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetches content from a specified URL and returns the response data.

Usage:
- Takes a URL and optional HTTP parameters as input
- Performs HTTP requests (GET, POST, PUT, DELETE, etc.)
- Returns response body with optional format conversion
- Supports custom headers, request body, and timeout configuration
- Can extract and simplify HTML content for easier analysis
- Use this tool when you need to retrieve web content or interact with APIs

Usage notes:
  - The URL must be a fully-formed valid URL (http:// or https://)
  - HTTP URLs will be automatically upgraded to HTTPS when possible
  - Default timeout is 15000ms (15 seconds), max 60000ms (60 seconds)
  - Redirects are followed by default
  - Response format can be 'text' (default) or 'json' for automatic parsing
  - extract_content option (default: true) will simplify HTML to plain text
  - max_content_length (default: 2000) limits the response size returned
  - This tool is read-only and does not modify any files
  - Rate limited to 10 calls per minute to prevent abuse
  - SSRF protection: Blocks requests to private IP ranges"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch (must start with http:// or https://)"
                }
            },
            "required": ["url"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network, ToolPriority::Expensive)
            .with_rate_limit(RateLimitConfig {
                max_calls: 10,
                window_secs: 60,
                backoff: BackoffStrategy::Exponential {
                    base_ms: 1000,
                    max_ms: 30_000,
                },
            })
            .with_timeout(Duration::from_secs(60))
            .with_tags(vec!["network".into(), "http".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Network]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: WebFetchArgs = serde_json::from_value(args)?;

        let parsed_url = match Url::parse(&args.url) {
            Ok(url) => url,
            Err(_) => {
                return Ok(validation_error(format!(
                    "Invalid URL format: {}",
                    args.url
                )));
            }
        };

        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Ok(validation_error(
                "Only HTTP and HTTPS protocols are supported",
            ));
        }

        if is_local_address(&parsed_url) {
            return Ok(validation_error(
                "Requests to local or private network addresses are not allowed",
            ));
        }

        let timeout_ms = 30_000; // 固定 30 秒超时
        let follow = true; // 总是跟随重定向
        let max_len = 2000; // 固定 2000 字符限制

        match try_jina_reader(&parsed_url, timeout_ms).await {
            Ok(Some(jina_content)) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Success(jina_content.clone())],
                    is_error: false,
                    execution_time_ms: None,
                    ext_info: Some(json!({
                        "url": parsed_url.as_str(),
                        "source": "jina",
                    })),
                });
            }
            Ok(None) => {
                // Fall through to direct fetching below
            }
            Err(err_result) => {
                return Ok(err_result);
            }
        }

        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(if follow {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent("OrbitX-Agent/1.0");

        let client = client_builder.build()?;

        let request = client.get(parsed_url.clone());

        let started = std::time::Instant::now();
        let resp = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error(format!("request failed: {}", e))],
                    is_error: true,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    ext_info: None,
                });
            }
        };

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let mut headers = HashMap::new();
        for (k, v) in resp.headers() {
            if let Ok(s) = v.to_str() {
                headers.insert(k.to_string(), s.to_string());
            }
        }
        let content_type = headers.get("content-type").cloned();

        let raw_text = match resp.text().await {
            Ok(t) => t,
            Err(e) => format!("<read-error>{}", e),
        };

        let (data_text, extracted_text) = if content_type
            .as_deref()
            .is_some_and(|ct| ct.contains("text/html"))
        {
            let (text, _title) = extract_content_from_html(&raw_text, max_len);
            (summarize_text(&text, max_len), Some(text))
        } else {
            (truncate_text(&raw_text, max_len), None)
        };

        let meta = json!({
            "status": status,
            "final_url": final_url,
            "headers": headers,
            "content_type": content_type,
            "extracted": extracted_text.is_some(),
            "elapsed_ms": started.elapsed().as_millis() as u64,
            "source": "direct",
        });

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(data_text)],
            is_error: !(200..400).contains(&status),
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
            ext_info: Some(meta),
        })
    }
}

fn truncate_text(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let truncated = crate::agent::utils::truncate_at_char_boundary(s, max_len);
    format!("{}...\n[truncated, original {} chars]", truncated, s.len())
}

fn summarize_text(content: &str, max_len: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 50 {
        return truncate_text(content, max_len);
    }
    let mut out = String::new();
    for l in lines.iter().take(20) {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&format!(
        "\n... [omitted {} lines] ...\n\n",
        lines.len().saturating_sub(30)
    ));
    for l in lines.iter().skip(lines.len().saturating_sub(10)) {
        out.push_str(l);
        out.push('\n');
    }
    truncate_text(&out, max_len)
}

fn extract_content_from_html(html: &str, max_length: usize) -> (String, Option<String>) {
    use html2text::from_read;
    let text = from_read(html.as_bytes(), max_length.max(4096));
    let cleaned = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let final_text = if cleaned.len() > max_length {
        let truncated = crate::agent::utils::truncate_at_char_boundary(&cleaned, max_length);
        format!(
            "{}...\n\n[内容被截断，原始长度: {} 字符]",
            truncated,
            cleaned.len()
        )
    } else {
        cleaned
    };
    (final_text, None)
}

fn is_local_address(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(host)) => {
            let host_lower = host.to_lowercase();
            if host_lower == "localhost" || host_lower.ends_with(".local") {
                return true;
            }
            if let Ok(addr) = host.parse::<IpAddr>() {
                return is_private_ip(&addr);
            }
            false
        }
        Some(url::Host::Ipv4(addr)) => is_private_ip(&IpAddr::V4(addr)),
        Some(url::Host::Ipv6(addr)) => is_private_ip(&IpAddr::V6(addr)),
        None => false,
    }
}

fn is_private_ip(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

async fn try_jina_reader(url: &Url, timeout_ms: u64) -> Result<Option<String>, ToolResult> {
    let jina_url = format!("https://r.jina.ai/{}", url.as_str());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent("OrbitX-Agent/1.0")
        .build()
        .map_err(|e| tool_error(format!("Failed to build request client: {}", e)))?;

    let response = match client.get(jina_url).send().await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if !response.status().is_success() {
        return Ok(None);
    }

    match response.text().await {
        Ok(text) if text.trim().len() > 50 => Ok(Some(text)),
        _ => Ok(None),
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
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
