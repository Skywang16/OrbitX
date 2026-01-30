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
use tokio::net::lookup_host;
use url::Url;

use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    BackoffStrategy, RateLimitConfig, RunnableTool, ToolCategory, ToolMetadata, ToolPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};

#[derive(Debug, Deserialize)]
struct WebFetchArgs {
    url: String,
}

pub struct WebFetchTool;
impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        r#"Fetches content from a URL and returns it as text.

Usage:
- Takes a URL as input and performs an HTTP GET request
- HTML content is automatically converted to readable plain text
- JSON responses are returned as-is
- Returns at most 2000 characters; large responses are truncated
- The URL must be a fully-formed valid URL starting with http:// or https://
- Blocked: localhost, private IPs, and internal network addresses
- This tool is read-only and does not modify any files"#
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
            .with_confirmation()
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

        if let Err(err) = validate_fetch_url(&parsed_url).await {
            return Ok(validation_error(err.to_string()));
        }

        let timeout_ms = 30_000; // 固定 30 秒超时
        let max_len = 2000; // 固定 2000 字符限制

        match try_jina_reader(&parsed_url, timeout_ms).await {
            Ok(Some(jina_content)) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Success(jina_content.clone())],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
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
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("OrbitX-Agent/1.0");

        let client = client_builder.build()?;

        let started = std::time::Instant::now();
        let resp = match fetch_follow_redirects(&client, parsed_url.clone(), 10).await {
            Ok(r) => r,
            Err(err) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error(err.to_string())],
                    status: ToolResultStatus::Error,
                    cancel_reason: None,
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
            Err(e) => format!("<read-error>{e}"),
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

        let status_flag = if (200..400).contains(&status) {
            ToolResultStatus::Success
        } else {
            ToolResultStatus::Error
        };

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(data_text)],
            status: status_flag,
            cancel_reason: None,
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

fn is_private_ip(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

async fn validate_fetch_url(url: &Url) -> ToolExecutorResult<()> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ToolExecutorError::InvalidArguments {
            tool_name: "web_fetch".to_string(),
            error: "Only HTTP and HTTPS protocols are supported".to_string(),
        });
    }

    let host = url
        .host()
        .ok_or_else(|| ToolExecutorError::InvalidArguments {
            tool_name: "web_fetch".to_string(),
            error: "URL host is missing".to_string(),
        })?;
    let port = url.port_or_known_default().unwrap_or(80);

    match host {
        url::Host::Ipv4(addr) => {
            if is_private_ip(&IpAddr::V4(addr)) {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "web_fetch".to_string(),
                    error: "Requests to local or private network addresses are not allowed"
                        .to_string(),
                });
            }
        }
        url::Host::Ipv6(addr) => {
            if is_private_ip(&IpAddr::V6(addr)) {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "web_fetch".to_string(),
                    error: "Requests to local or private network addresses are not allowed"
                        .to_string(),
                });
            }
        }
        url::Host::Domain(host) => {
            let host_lower = host.to_lowercase();
            if host_lower == "localhost" || host_lower.ends_with(".local") {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "web_fetch".to_string(),
                    error: "Requests to local or private network addresses are not allowed"
                        .to_string(),
                });
            }

            let addrs = lookup_host((host, port)).await.map_err(|e| {
                ToolExecutorError::ExecutionFailed {
                    tool_name: "web_fetch".to_string(),
                    error: format!("Failed to resolve host '{host}': {e}"),
                }
            })?;
            for addr in addrs {
                if is_private_ip(&addr.ip()) {
                    return Err(ToolExecutorError::InvalidArguments {
                        tool_name: "web_fetch".to_string(),
                        error: "Requests to local or private network addresses are not allowed"
                            .to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

async fn fetch_follow_redirects(
    client: &reqwest::Client,
    mut url: Url,
    max_redirects: usize,
) -> ToolExecutorResult<reqwest::Response> {
    for _ in 0..=max_redirects {
        validate_fetch_url(&url).await?;

        let resp = client.get(url.clone()).send().await.map_err(|e| {
            ToolExecutorError::ExecutionFailed {
                tool_name: "web_fetch".to_string(),
                error: format!("request failed: {e}"),
            }
        })?;

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());

            if let Some(location) = location {
                url = url
                    .join(location)
                    .map_err(|e| ToolExecutorError::InvalidArguments {
                        tool_name: "web_fetch".to_string(),
                        error: format!("Invalid redirect URL: {e}"),
                    })?;
                continue;
            }
        }

        return Ok(resp);
    }

    Err(ToolExecutorError::ResourceLimitExceeded {
        tool_name: "web_fetch".to_string(),
        resource_type: format!("too many redirects (max: {max_redirects})"),
    })
}

async fn try_jina_reader(url: &Url, timeout_ms: u64) -> Result<Option<String>, ToolResult> {
    let jina_url = format!("https://r.jina.ai/{}", url.as_str());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent("OrbitX-Agent/1.0")
        .build()
        .map_err(|e| tool_error(format!("Failed to build request client: {e}")))?;

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
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}
