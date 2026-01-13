use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::agent::mcp::transport::stdio::{
    ensure_abs_workspace, expand_args, expand_env_map, StdioTransport,
};
use crate::agent::mcp::transport::streamable_http::StreamableHttpTransport;
use crate::agent::mcp::transport::McpTransport;
use crate::agent::mcp::types::{McpToolDefinition, ServerInfo, ToolCallResult};
use crate::settings::types::McpServerConfig;

pub struct McpClient {
    name: String,
    transport: Arc<dyn McpTransport>,
    server_info: Option<ServerInfo>,
    tools: Vec<McpToolDefinition>,
}

impl McpClient {
    pub async fn new(
        name: String,
        config: &McpServerConfig,
        workspace_root: &Path,
    ) -> McpResult<Self> {
        let transport: Arc<dyn McpTransport> = match config {
            McpServerConfig::Stdio {
                command,
                args,
                env,
                disabled,
            } => {
                if *disabled {
                    return Err(McpError::Disabled);
                }
                let workspace_root = ensure_abs_workspace(workspace_root)?;
                let args = expand_args(args, &workspace_root);
                let env = expand_env_map(env, &workspace_root);
                Arc::new(StdioTransport::spawn(command, &args, &env, Some(&workspace_root)).await?)
            }
            McpServerConfig::Sse { disabled, .. } => {
                if *disabled {
                    return Err(McpError::Disabled);
                }
                return Err(McpError::InvalidConfig(
                    "SSE MCP servers not supported yet".into(),
                ));
            }
            McpServerConfig::StreamableHttp {
                url,
                headers,
                disabled,
            } => {
                if *disabled {
                    return Err(McpError::Disabled);
                }
                Arc::new(StreamableHttpTransport::new(url.clone(), headers)?)
            }
        };

        let mut client = Self {
            name,
            transport,
            server_info: None,
            tools: Vec::new(),
        };

        client.initialize().await?;
        client.refresh_tools().await?;

        Ok(client)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tools(&self) -> &[McpToolDefinition] {
        &self.tools
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> McpResult<ToolCallResult> {
        let resp = self
            .transport
            .request(JsonRpcRequest::new_request(
                0,
                "tools/call",
                Some(json!({
                    "name": name,
                    "arguments": arguments
                })),
            ))
            .await?;

        parse_result(resp, "tools/call")
    }

    async fn initialize(&mut self) -> McpResult<()> {
        let resp = self
            .transport
            .request(JsonRpcRequest::new_request(
                0,
                "initialize",
                Some(json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": { "name": "OrbitX", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": {}
                })),
            ))
            .await?;

        let result = parse_json_result(resp, "initialize")?;
        if let Some(info) = result.get("serverInfo") {
            let server_info: ServerInfo = serde_json::from_value(info.clone())?;
            self.server_info = Some(server_info);
        }

        let _ = self
            .transport
            .notify(JsonRpcRequest::new_notification("initialized", None))
            .await;

        Ok(())
    }

    async fn refresh_tools(&mut self) -> McpResult<()> {
        let resp = self
            .transport
            .request(JsonRpcRequest::new_request(0, "tools/list", None))
            .await?;

        let result = parse_json_result(resp, "tools/list")?;
        let tools_val = result
            .get("tools")
            .ok_or_else(|| McpError::Protocol("tools/list missing tools".into()))?;

        let tools: Vec<McpToolDefinition> = serde_json::from_value(tools_val.clone())?;
        self.tools = tools;
        Ok(())
    }
}

fn parse_json_result(resp: JsonRpcResponse, method: &str) -> McpResult<Value> {
    if let Some(err) = resp.error {
        return Err(McpError::Protocol(format!(
            "{method} error (code={}): {}",
            err.code, err.message
        )));
    }
    resp.result
        .ok_or_else(|| McpError::Protocol(format!("{method} missing result")))
}

fn parse_result(resp: JsonRpcResponse, method: &str) -> McpResult<ToolCallResult> {
    let result = parse_json_result(resp, method)?;
    let call: ToolCallResult = serde_json::from_value(result)?;
    Ok(call)
}
