use async_trait::async_trait;
use reqwest::{Client, header};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use crate::agent::mcp::transport::McpTransport;

/// Streamable HTTP transport for MCP (2025-03-26 spec).
///
/// Uses HTTP POST for requests and notifications.
/// Server may respond with JSON-RPC directly or use SSE for streaming.
pub struct StreamableHttpTransport {
    client: Client,
    url: String,
    request_id: AtomicI64,
}

impl StreamableHttpTransport {
    pub fn new(url: String, headers: &HashMap<String, String>) -> McpResult<Self> {
        let mut header_map = header::HeaderMap::new();
        header_map.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        header_map.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json, text/event-stream"),
        );

        for (key, value) in headers {
            let name = header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| McpError::InvalidConfig(format!("Invalid header name: {e}")))?;
            let val = header::HeaderValue::from_str(value)
                .map_err(|e| McpError::InvalidConfig(format!("Invalid header value: {e}")))?;
            header_map.insert(name, val);
        }

        let client = Client::builder()
            .default_headers(header_map)
            .build()
            .map_err(|e| McpError::Protocol(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self {
            client,
            url,
            request_id: AtomicI64::new(1),
        })
    }

    fn next_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }
}

#[async_trait]
impl McpTransport for StreamableHttpTransport {
    async fn request(&self, mut request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        // Assign request ID if not set
        if request.id.is_none() {
            request.id = Some(JsonRpcId::Number(self.next_id()));
        }

        let body = serde_json::to_string(&request)
            .map_err(|e| McpError::Protocol(format!("Failed to serialize request: {e}")))?;

        let response = self
            .client
            .post(&self.url)
            .body(body)
            .send()
            .await
            .map_err(|e| McpError::Protocol(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(McpError::Protocol(format!("HTTP error {status}: {text}")));
        }

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("text/event-stream") {
            // Handle SSE response - read events until we get a response
            let text = response
                .text()
                .await
                .map_err(|e| McpError::Protocol(format!("Failed to read SSE response: {e}")))?;

            // Parse SSE events to find the JSON-RPC response
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(data) {
                        return Ok(resp);
                    }
                }
            }

            Err(McpError::Protocol(
                "No JSON-RPC response in SSE stream".into(),
            ))
        } else {
            // Standard JSON response
            let text = response
                .text()
                .await
                .map_err(|e| McpError::Protocol(format!("Failed to read response body: {e}")))?;

            serde_json::from_str(&text).map_err(|e| {
                McpError::Protocol(format!("Failed to parse response: {e}, body: {text}"))
            })
        }
    }

    async fn notify(&self, request: JsonRpcRequest) -> McpResult<()> {
        let body = serde_json::to_string(&request)
            .map_err(|e| McpError::Protocol(format!("Failed to serialize notification: {e}")))?;

        let response = self
            .client
            .post(&self.url)
            .body(body)
            .send()
            .await
            .map_err(|e| McpError::Protocol(format!("HTTP notification failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(McpError::Protocol(format!(
                "HTTP notification error {status}: {text}"
            )));
        }

        Ok(())
    }

    async fn close(&self) -> McpResult<()> {
        // HTTP is stateless, nothing to close
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // HTTP is stateless, always "connected"
        true
    }
}
