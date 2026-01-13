use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::agent::mcp::adapter::McpToolAdapter;
use crate::agent::mcp::client::McpClient;
use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::types::{
    McpConnectionStatus, McpServerSource, McpServerStatus, McpToolInfo,
};
use crate::settings::types::{EffectiveSettings, McpServerConfig, Settings};

/// 存储单个客户端及其元信息
struct ClientEntry {
    source: McpServerSource,
    status: McpConnectionStatus,
    error: Option<String>,
    client: Option<Arc<McpClient>>,
}

#[derive(Default)]
pub struct McpRegistry {
    /// 工作区特定的MCP服务器
    workspace_clients: DashMap<PathBuf, DashMap<String, ClientEntry>>,
}

impl McpRegistry {
    /// 初始化工作区 MCP 服务器（effective = 全局+工作区合并结果）
    pub async fn init_workspace_servers(
        &self,
        workspace_root: &Path,
        effective: &EffectiveSettings,
        workspace_settings: Option<&Settings>,
    ) -> McpResult<()> {
        if !workspace_root.is_absolute() {
            return Err(McpError::WorkspaceNotAbsolute(workspace_root.to_path_buf()));
        }

        let map = DashMap::<String, ClientEntry>::new();

        for (name, config) in effective.mcp_servers.iter() {
            if is_disabled(config) {
                continue;
            }

            let source = if workspace_settings
                .and_then(|s| s.mcp_servers.get(name))
                .is_some()
            {
                McpServerSource::Workspace
            } else {
                McpServerSource::Global
            };

            match McpClient::new(name.clone(), config, workspace_root).await {
                Ok(client) => {
                    map.insert(
                        name.clone(),
                        ClientEntry {
                            source,
                            status: McpConnectionStatus::Connected,
                            error: None,
                            client: Some(Arc::new(client)),
                        },
                    );
                }
                Err(McpError::Disabled) => continue,
                Err(e) => {
                    tracing::warn!(target: "mcp", server = %name, error = %e, "Failed to init MCP server");
                    map.insert(
                        name.clone(),
                        ClientEntry {
                            source,
                            status: McpConnectionStatus::Error,
                            error: Some(e.to_string()),
                            client: None,
                        },
                    );
                }
            }
        }

        self.workspace_clients
            .insert(workspace_root.to_path_buf(), map);
        Ok(())
    }

    /// 获取工作区所有可用工具
    pub fn get_tools_for_workspace(&self, workspace_root: &Path) -> Vec<McpToolAdapter> {
        let mut out = Vec::new();

        if let Some(workspace_servers) = self.workspace_clients.get(workspace_root) {
            for entry in workspace_servers.iter() {
                let Some(client) = entry.value().client.as_ref() else {
                    continue;
                };
                for tool in client.tools().iter().cloned() {
                    out.push(McpToolAdapter::new(Arc::clone(client), tool));
                }
            }
        }

        out
    }

    /// 获取所有服务器状态（用于前端显示）
    pub fn get_servers_status(&self, workspace_root: Option<&Path>) -> Vec<McpServerStatus> {
        let mut statuses = Vec::new();

        if let Some(workspace) = workspace_root {
            if let Some(workspace_servers) = self.workspace_clients.get(workspace) {
                for entry in workspace_servers.iter() {
                    let name = entry.key().clone();
                    let client_entry = entry.value();
                    let tools: Vec<McpToolInfo> = client_entry
                        .client
                        .as_ref()
                        .map(|c| {
                            c.tools()
                                .iter()
                                .map(|t| McpToolInfo {
                                    name: t.name.clone(),
                                    description: if t.description.is_empty() {
                                        None
                                    } else {
                                        Some(t.description.clone())
                                    },
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    statuses.push(McpServerStatus {
                        name,
                        source: client_entry.source,
                        status: client_entry.status,
                        tools,
                        error: client_entry.error.clone(),
                    });
                }
            }
        }

        statuses
    }

    /// 重新加载工作区服务器
    pub async fn reload_workspace_servers(
        &self,
        workspace_root: &Path,
        effective: &EffectiveSettings,
        workspace_settings: Option<&Settings>,
    ) -> McpResult<()> {
        // 移除旧的工作区客户端
        self.workspace_clients.remove(workspace_root);
        // 重新初始化
        self.init_workspace_servers(workspace_root, effective, workspace_settings)
            .await
    }

    /// 获取工作区服务器数量
    pub fn workspace_server_count(&self, workspace_root: &Path) -> usize {
        self.workspace_clients
            .get(workspace_root)
            .map(|m| m.len())
            .unwrap_or(0)
    }
}

fn is_disabled(config: &McpServerConfig) -> bool {
    match config {
        McpServerConfig::Stdio { disabled, .. } => *disabled,
        McpServerConfig::Sse { disabled, .. } => *disabled,
        McpServerConfig::StreamableHttp { disabled, .. } => *disabled,
    }
}
