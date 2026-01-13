use async_trait::async_trait;

use crate::agent::mcp::error::McpResult;
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};

pub mod sse;
pub mod stdio;
pub mod streamable_http;

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn request(&self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse>;
    async fn notify(&self, request: JsonRpcRequest) -> McpResult<()>;
    async fn close(&self) -> McpResult<()>;
    fn is_connected(&self) -> bool;
}

