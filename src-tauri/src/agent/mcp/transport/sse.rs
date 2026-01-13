use async_trait::async_trait;

use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::agent::mcp::transport::McpTransport;

/// SSE transport skeleton.
///
/// OrbitX currently focuses on stdio MCP servers. SSE will be implemented once we have
/// at least one real server to validate against.
pub struct SseTransport;

#[async_trait]
impl McpTransport for SseTransport {
    async fn request(&self, _request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        Err(McpError::InvalidConfig(
            "SSE transport not implemented yet".into(),
        ))
    }

    async fn notify(&self, _request: JsonRpcRequest) -> McpResult<()> {
        Err(McpError::InvalidConfig(
            "SSE transport not implemented yet".into(),
        ))
    }

    async fn close(&self) -> McpResult<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        false
    }
}
