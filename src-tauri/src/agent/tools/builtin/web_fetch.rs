/*!
 * Web Fetch Tool
 *
 * Provides headless HTTP requests as an Agent tool so LLM can call it via tool-calls.
 */

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolPermission, ToolResult, ToolResultContent,
};

#[derive(Debug, Deserialize)]
struct WebFetchArgs {
    url: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    headers: Option<HashMap<String, String>>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default)]
    follow_redirects: Option<bool>,
    #[serde(default)]
    response_format: Option<String>, // "text" | "json"
    #[serde(default)]
    extract_content: Option<bool>,
    #[serde(default)]
    max_content_length: Option<usize>,
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
        "Perform a headless HTTP request and return response data (optionally summarized)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Target URL (http/https)" },
                "method": { "type": "string", "default": "GET" },
                "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                "body": { "type": "string" },
                "timeout": { "type": "integer", "description": "Timeout millis (default 15000)" },
                "follow_redirects": { "type": "boolean", "default": true },
                "response_format": { "type": "string", "enum": ["text", "json"], "default": "text" },
                "extract_content": { "type": "boolean", "default": true },
                "max_content_length": { "type": "integer", "default": 2000 }
            },
            "required": ["url"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::Network]
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "network".to_string(),
            "http".to_string(),
            "fetch".to_string(),
        ]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: WebFetchArgs = serde_json::from_value(args)?;

        let method = args.method.as_deref().unwrap_or("GET").to_uppercase();
        let timeout_ms = args.timeout.unwrap_or(15_000);
        let follow = args.follow_redirects.unwrap_or(true);
        let response_format = args.response_format.as_deref().unwrap_or("text");
        let extract_content = args.extract_content.unwrap_or(true);
        let max_len = args.max_content_length.unwrap_or(2000);

        // Build client
        #[cfg(debug_assertions)]
        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(if follow {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent("OrbitX-Agent/1.0")
            .danger_accept_invalid_certs(true);

        #[cfg(not(debug_assertions))]
        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(if follow {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent("OrbitX-Agent/1.0");

        let client = client_builder.build()?;

        // Build request
        let req_method = match method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => reqwest::Method::GET,
        };

        let mut request = client.request(req_method, &args.url);
        if let Some(h) = &args.headers {
            for (k, v) in h {
                request = request.header(k, v);
            }
        }
        if let Some(body) = &args.body {
            request = request.body(body.clone());
        }

        let started = std::time::Instant::now();
        let resp = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error {
                        message: format!("request failed: {}", e),
                        details: Some(args.url),
                    }],
                    is_error: true,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    metadata: None,
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

        let (data_text, extracted_text) = if extract_content
            && content_type
                .as_deref()
                .is_some_and(|ct| ct.contains("text/html"))
        {
            let (text, _title) = extract_content_from_html(&raw_text, max_len);
            (summarize_text(&text, max_len), Some(text))
        } else {
            let t = if response_format == "json" {
                match serde_json::from_str::<serde_json::Value>(&raw_text) {
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(raw_text.clone()),
                    Err(_) => raw_text.clone(),
                }
            } else {
                raw_text.clone()
            };
            (truncate_text(&t, max_len), None)
        };

        let meta = json!({
            "status": status,
            "final_url": final_url,
            "headers": headers,
            "content_type": content_type,
            "extracted": extracted_text.is_some(),
            "elapsed_ms": started.elapsed().as_millis() as u64,
        });

        Ok(ToolResult {
            content: vec![ToolResultContent::Text { text: data_text }],
            is_error: !(200..400).contains(&status),
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
            metadata: Some(meta),
        })
    }
}

fn truncate_text(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    format!(
        "{}...\n[truncated, original {} chars]",
        &s[..max_len],
        s.len()
    )
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
        format!(
            "{}...\n\n[内容被截断，原始长度: {} 字符]",
            &cleaned[..max_length],
            cleaned.len()
        )
    } else {
        cleaned
    };
    (final_text, None)
}
