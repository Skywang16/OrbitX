/*!
 * Web Fetch Tool
 *
 * Claude Code-style web fetcher: accepts a URL and a prompt (question),
 * fetches the page, converts HTML→Markdown (htmd / Turndown-style),
 * then uses the main LLM to produce a concise answer grounded in the
 * page content. Results are cached for 15 minutes.
 */

use async_trait::async_trait;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::lookup_host;
use url::Url;

use crate::agent::common::llm_text::extract_text_from_llm_message;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    BackoffStrategy, RateLimitConfig, RunnableTool, ToolCategory, ToolMetadata, ToolPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};
use crate::llm::anthropic_types::{
    CreateMessageRequest, MessageContent, MessageParam, MessageRole,
};
use crate::llm::service::LLMService;

// ---------------------------------------------------------------------------
// Shared HTTP client (connection-pooled, reused across all fetches)
// ---------------------------------------------------------------------------
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("OrbitX-Agent/1.0")
        .pool_max_idle_per_host(4)
        .build()
        .expect("failed to build shared HTTP client")
});

// ---------------------------------------------------------------------------
// URL content cache — 15-minute TTL, keyed by normalized URL string
// ---------------------------------------------------------------------------
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_CACHE_ENTRIES: usize = 64;
const MAX_CONTENT_BYTES: usize = 100 * 1024; // 100 KB (same as Claude Code)
const MAX_HTML_INPUT_BYTES: usize = 512 * 1024;

struct CachedPage {
    markdown: String,
    fetched_at: Instant,
}

static PAGE_CACHE: Lazy<DashMap<String, CachedPage>> = Lazy::new(DashMap::new);
static SCRIPT_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script>").expect("valid script regex"));
static STYLE_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<style\b[^>]*>.*?</style>").expect("valid style regex"));
static NOSCRIPT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<noscript\b[^>]*>.*?</noscript>").expect("valid noscript regex")
});
static SVG_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<svg\b[^>]*>.*?</svg>").expect("valid svg regex"));
static COMMENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<!--.*?-->").expect("valid html comment regex"));
static MAIN_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<main\b[^>]*>(.*?)</main>").expect("valid main regex"));
static ARTICLE_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<article\b[^>]*>(.*?)</article>").expect("valid article regex"));
static BODY_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<body\b[^>]*>(.*?)</body>").expect("valid body regex"));

fn cache_get(url: &str) -> Option<String> {
    if let Some(entry) = PAGE_CACHE.get(url) {
        if entry.fetched_at.elapsed() < CACHE_TTL {
            return Some(entry.markdown.clone());
        }
        drop(entry);
        PAGE_CACHE.remove(url);
    }
    None
}

fn cache_put(url: String, markdown: String) {
    // Evict expired entries when cache is getting full
    if PAGE_CACHE.len() >= MAX_CACHE_ENTRIES {
        cache_evict_expired();
    }
    // If still at capacity after eviction, remove oldest entry
    if PAGE_CACHE.len() >= MAX_CACHE_ENTRIES {
        if let Some(oldest_key) = PAGE_CACHE
            .iter()
            .min_by_key(|entry| entry.value().fetched_at)
            .map(|entry| entry.key().clone())
        {
            PAGE_CACHE.remove(&oldest_key);
        }
    }
    PAGE_CACHE.insert(
        url,
        CachedPage {
            markdown,
            fetched_at: Instant::now(),
        },
    );
}

fn cache_evict_expired() {
    let expired: Vec<String> = PAGE_CACHE
        .iter()
        .filter(|entry| entry.value().fetched_at.elapsed() >= CACHE_TTL)
        .map(|entry| entry.key().clone())
        .collect();
    for key in expired {
        PAGE_CACHE.remove(&key);
    }
}

// ---------------------------------------------------------------------------
// Tool definition
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
struct WebFetchArgs {
    url: String,
    prompt: Option<String>,
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
        r#"Fetch a web page and answer a question about its content.

Usage:
- Provide `url` (required) and `prompt` (recommended) — the question you want answered from the page.
- HTML pages are converted to Markdown; the LLM then extracts a concise answer based on your prompt.
- If `prompt` is omitted, the raw Markdown content is returned (truncated to 100 KB).
- Results are cached for 15 minutes; repeated fetches of the same URL are instant.
- The URL must start with http:// or https://. Localhost and private IPs are blocked.

Best Practices:
- Always provide a specific `prompt` to get focused, relevant answers instead of raw page dumps.
- When web_search returns results, use web_fetch with a targeted prompt to read the most relevant pages.
- After fetching, if you discover additional relevant URLs, fetch those too.

Examples:
- {"url": "https://docs.rs/tokio/latest", "prompt": "What async runtime features does tokio provide?"}
- {"url": "https://react.dev/reference/react/useState", "prompt": "What are the rules for calling useState?"}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch (must start with http:// or https://)",
                    "maxLength": 2000
                },
                "prompt": {
                    "type": "string",
                    "description": "The question to answer from the fetched page content"
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
            .with_timeout(Duration::from_secs(90))
            .with_tags(vec!["network".into(), "http".into()])
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        use tracing::{debug, info};

        let args: WebFetchArgs = serde_json::from_value(args)?;
        let url_str = args.url.trim().to_string();
        let user_query = args.prompt.unwrap_or_default();

        info!("WebFetch: url={} prompt_len={}", url_str, user_query.len());

        // --- URL validation ---
        let parsed_url = match Url::parse(&url_str) {
            Ok(u) => u,
            Err(_) => return Ok(validation_error(format!("Invalid URL: {url_str}"))),
        };
        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Ok(validation_error("Only HTTP/HTTPS supported"));
        }
        if let Err(e) = validate_fetch_url(&parsed_url).await {
            return Ok(validation_error(e.to_string()));
        }

        let started = Instant::now();
        let mut fetch_ms: Option<u64> = None;
        let mut body_read_ms: Option<u64> = None;
        let mut convert_ms: Option<u64> = None;
        let mut cache_hit = false;

        // --- Fetch (with cache) ---
        let markdown = if let Some(cached) = cache_get(&url_str) {
            debug!("Cache hit for {}", url_str);
            cache_hit = true;
            cached
        } else {
            debug!("Cache miss, fetching {}", url_str);
            let fetch_started = Instant::now();
            let resp = match fetch_follow_redirects(&HTTP_CLIENT, parsed_url.clone(), 10).await {
                Ok(r) => r,
                Err(e) => {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Error(e.to_string())],
                        status: ToolResultStatus::Error,
                        cancel_reason: None,
                        execution_time_ms: Some(started.elapsed().as_millis() as u64),
                        ext_info: None,
                    });
                }
            };
            fetch_ms = Some(fetch_started.elapsed().as_millis() as u64);

            let status = resp.status().as_u16();
            if !(200..300).contains(&status) {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error(format!("HTTP {status}"))],
                    status: ToolResultStatus::Error,
                    cancel_reason: None,
                    execution_time_ms: Some(started.elapsed().as_millis() as u64),
                    ext_info: None,
                });
            }

            let content_type = resp
                .headers()
                .get("content-type")
                .map(|value| {
                    value
                        .to_str()
                        .map(|value| value.to_string())
                        .map_err(|err| format!("Invalid content-type header: {err}"))
                })
                .transpose();
            let content_type = match content_type {
                Ok(Some(content_type)) => content_type,
                Ok(None) => String::new(),
                Err(err) => return Ok(validation_error(err)),
            };

            let body_started = Instant::now();
            let raw = match tokio::time::timeout(Duration::from_secs(30), resp.text()).await {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => return Ok(validation_error(format!("Body read error: {e}"))),
                Err(_) => return Ok(validation_error("Body read timeout")),
            };
            body_read_ms = Some(body_started.elapsed().as_millis() as u64);

            let md = if content_type.contains("text/html") {
                let convert_started = Instant::now();
                let converted =
                    html_to_markdown(&raw).map_err(|err| ToolExecutorError::ExecutionFailed {
                        tool_name: "web_fetch".to_string(),
                        error: err,
                    })?;
                convert_ms = Some(convert_started.elapsed().as_millis() as u64);
                converted
            } else {
                raw
            };

            let md = truncate_content(&md, MAX_CONTENT_BYTES);
            cache_put(url_str.clone(), md.clone());
            md
        };

        // --- If no prompt, return raw markdown ---
        if user_query.trim().is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(markdown)],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: Some(started.elapsed().as_millis() as u64),
                ext_info: Some(json!({
                    "source": url_str,
                    "summarized": false,
                    "cacheHit": cache_hit,
                    "timings": {
                        "fetchMs": fetch_ms,
                        "bodyReadMs": body_read_ms,
                        "convertMs": convert_ms
                    }
                })),
            });
        }

        // --- LLM summarization (Claude Code style) ---
        let summarize_started = Instant::now();
        let summary = summarize_with_llm(context, &markdown, &user_query)
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "web_fetch".to_string(),
                error: format!("summarization failed: {e}"),
            })?;
        let summarize_ms = Some(summarize_started.elapsed().as_millis() as u64);

        info!(
            "WebFetch timings: url={} cache_hit={} fetch_ms={:?} body_ms={:?} convert_ms={:?} summarize_ms={:?} total_ms={}",
            url_str,
            cache_hit,
            fetch_ms,
            body_read_ms,
            convert_ms,
            summarize_ms,
            started.elapsed().as_millis()
        );

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(summary)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: Some(started.elapsed().as_millis() as u64),
            ext_info: Some(json!({
                "source": url_str,
                "summarized": true,
                "cacheHit": cache_hit,
                "timings": {
                    "fetchMs": fetch_ms,
                    "bodyReadMs": body_read_ms,
                    "convertMs": convert_ms,
                    "summarizeMs": summarize_ms
                }
            })),
        })
    }
}

// ---------------------------------------------------------------------------
// HTML → Markdown conversion (htmd, Turndown-style)
// ---------------------------------------------------------------------------
fn html_to_markdown(html: &str) -> Result<String, String> {
    let preprocessed = preprocess_html_for_markdown(html);
    htmd::convert(&preprocessed).map_err(|err| format!("HTML to Markdown conversion failed: {err}"))
}

fn preprocess_html_for_markdown(html: &str) -> String {
    let mut candidate = extract_html_focus_region(html);

    if candidate.len() > MAX_HTML_INPUT_BYTES {
        candidate =
            crate::agent::utils::truncate_at_char_boundary(&candidate, MAX_HTML_INPUT_BYTES)
                .to_string();
    }

    let candidate = SCRIPT_BLOCK_RE.replace_all(&candidate, "");
    let candidate = STYLE_BLOCK_RE.replace_all(&candidate, "");
    let candidate = NOSCRIPT_BLOCK_RE.replace_all(&candidate, "");
    let candidate = SVG_BLOCK_RE.replace_all(&candidate, "");
    let candidate = COMMENT_RE.replace_all(&candidate, "");

    candidate.into_owned()
}

fn extract_html_focus_region(html: &str) -> String {
    if let Some(captures) = MAIN_BLOCK_RE.captures(html) {
        if let Some(main) = captures.get(1) {
            return main.as_str().to_string();
        }
    }

    if let Some(captures) = ARTICLE_BLOCK_RE.captures(html) {
        if let Some(article) = captures.get(1) {
            return article.as_str().to_string();
        }
    }

    if let Some(captures) = BODY_BLOCK_RE.captures(html) {
        if let Some(body) = captures.get(1) {
            return body.as_str().to_string();
        }
    }

    html.to_string()
}

// ---------------------------------------------------------------------------
// LLM summarization — mirrors Claude Code's WebFetch prompt design
// ---------------------------------------------------------------------------
async fn summarize_with_llm(
    context: &AgentRunContext,
    content: &str,
    user_query: &str,
) -> Result<String, String> {
    let db = context.repositories();

    // Resolve model_id from the current thread
    let model_id = {
        let thread = context
            .agent_persistence()
            .threads()
            .get(context.thread_id)
            .await
            .map_err(|e| format!("thread lookup: {e}"))?
            .ok_or("thread not found")?;
        thread.model_id.ok_or("no model_id on thread")?
    };

    // Trim content to avoid blowing up the context window.
    // 80K chars ≈ ~20-25K tokens — leaves room for the answer.
    let trimmed = truncate_content(content, 80_000);

    let prompt = format!(
        "Web page content:\n---\n{trimmed}\n---\n\n{user_query}\n\n\
         Provide a concise response based only on the content above. In your response:\n\
         - Use quotation marks for exact language from the page; paraphrase everything else.\n\
         - Focus on technical details, code examples, and API signatures.\n\
         - If the page does not contain relevant information, say so clearly."
    );

    let request = CreateMessageRequest {
        model: model_id,
        max_tokens: 2048,
        system: None,
        developer_context: None,
        messages: vec![MessageParam {
            role: MessageRole::User,
            content: MessageContent::Text(prompt),
        }],
        tools: None,
        stream: false,
        temperature: Some(0.1),
        top_p: None,
        top_k: None,
        metadata: None,
        stop_sequences: None,
        thinking: None,
    };

    let llm = LLMService::new(Arc::clone(&db));
    let resp = llm
        .call(request)
        .await
        .map_err(|e| format!("LLM call failed: {e}"))?;

    Ok(extract_text_from_llm_message(&resp))
}

// ---------------------------------------------------------------------------
// Content truncation (char-boundary safe)
// ---------------------------------------------------------------------------
fn truncate_content(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let truncated = crate::agent::utils::truncate_at_char_boundary(s, max_len);
    format!(
        "{truncated}\n\n[Content truncated at {max_len} chars; original {} chars]",
        s.len()
    )
}

// ---------------------------------------------------------------------------
// URL / network validation (unchanged from original — SSRF protection)
// ---------------------------------------------------------------------------
fn is_private_ip(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

async fn validate_fetch_url(url: &Url) -> ToolExecutorResult<()> {
    use tracing::debug;

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
    let port = url
        .port_or_known_default()
        .ok_or_else(|| ToolExecutorError::InvalidArguments {
            tool_name: "web_fetch".to_string(),
            error: "URL port is unavailable for this scheme".to_string(),
        })?;

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

            debug!("Resolving host: {}", host);
            let addrs = tokio::time::timeout(Duration::from_secs(5), lookup_host((host, port)))
                .await
                .map_err(|_| ToolExecutorError::ExecutionFailed {
                    tool_name: "web_fetch".to_string(),
                    error: format!("DNS lookup timeout for host '{host}'"),
                })?
                .map_err(|e| ToolExecutorError::ExecutionFailed {
                    tool_name: "web_fetch".to_string(),
                    error: format!("Failed to resolve host '{host}': {e}"),
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
            debug!("Host validation passed for: {}", host);
        }
    }

    Ok(())
}

async fn fetch_follow_redirects(
    client: &reqwest::Client,
    mut url: Url,
    max_redirects: usize,
) -> ToolExecutorResult<reqwest::Response> {
    use tracing::{debug, warn};

    for redirect_count in 0..=max_redirects {
        validate_fetch_url(&url).await?;

        debug!("Fetching URL (redirect {}): {}", redirect_count, url);
        let resp = client.get(url.clone()).send().await.map_err(|e| {
            warn!("HTTP request failed for {}: {}", url, e);
            ToolExecutorError::ExecutionFailed {
                tool_name: "web_fetch".to_string(),
                error: format!("request failed: {e}"),
            }
        })?;

        debug!("Received response with status: {}", resp.status());

        if resp.status().is_redirection() {
            let location = match resp.headers().get(reqwest::header::LOCATION) {
                Some(value) => {
                    let location =
                        value
                            .to_str()
                            .map_err(|err| ToolExecutorError::ExecutionFailed {
                                tool_name: "web_fetch".to_string(),
                                error: format!("Invalid redirect location header: {err}"),
                            })?;
                    let trimmed = location.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                None => None,
            };

            if let Some(location) = location {
                debug!("Following redirect to: {}", location);
                url = url
                    .join(&location)
                    .map_err(|e| ToolExecutorError::InvalidArguments {
                        tool_name: "web_fetch".to_string(),
                        error: format!("Invalid redirect URL: {e}"),
                    })?;
                continue;
            }
        }

        return Ok(resp);
    }

    warn!("Too many redirects for URL: {}", url);
    Err(ToolExecutorError::ResourceLimitExceeded {
        tool_name: "web_fetch".to_string(),
        resource_type: format!("too many redirects (max: {max_redirects})"),
    })
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
