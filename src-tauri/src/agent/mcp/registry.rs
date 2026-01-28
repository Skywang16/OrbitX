use dashmap::DashMap;
use std::collections::BTreeMap;
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

struct WorkspaceMcpState {
    servers: BTreeMap<String, ClientEntry>,
}

#[derive(Default)]
pub struct McpRegistry {
    /// 工作区特定的MCP服务器
    workspaces: DashMap<Arc<str>, WorkspaceMcpState>,
}

impl McpRegistry {
    async fn canonicalize_workspace_root(&self, workspace_root: &Path) -> PathBuf {
        tokio::fs::canonicalize(workspace_root)
            .await
            .unwrap_or_else(|_| workspace_root.to_path_buf())
    }

    fn workspace_key(workspace_root: &Path) -> Arc<str> {
        Arc::from(workspace_root.to_string_lossy().to_string())
    }

    /// 初始化工作区 MCP 服务器（effective = 全局+工作区合并结果）
    pub async fn init_workspace_servers(
        &self,
        workspace_root: &Path,
        effective: &EffectiveSettings,
        workspace_settings: Option<&Settings>,
    ) -> McpResult<()> {
        let workspace_root = self.canonicalize_workspace_root(workspace_root).await;
        if !workspace_root.is_absolute() {
            return Err(McpError::WorkspaceNotAbsolute(workspace_root));
        }

        let mut servers = BTreeMap::<String, ClientEntry>::new();

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

            match McpClient::new(name.clone(), config, &workspace_root).await {
                Ok(client) => {
                    servers.insert(
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
                    servers.insert(
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

        let workspace_key = Self::workspace_key(&workspace_root);
        self.workspaces
            .insert(workspace_key, WorkspaceMcpState { servers });
        Ok(())
    }

    /// 获取工作区所有可用工具（workspace_key = canonical workspace path string）
    pub fn get_tools_for_workspace(&self, workspace_key: &str) -> Vec<McpToolAdapter> {
        let mut out = Vec::new();

        if let Some(workspace) = self.workspaces.get(workspace_key) {
            for entry in workspace.value().servers.values() {
                let Some(client) = entry.client.as_ref() else {
                    continue;
                };
                out.extend(
                    client
                        .tools()
                        .iter()
                        .cloned()
                        .map(|tool| McpToolAdapter::new(Arc::clone(client), tool)),
                );
            }
        }

        out
    }

    /// 获取所有服务器状态（用于前端显示）
    pub fn get_servers_status(&self, workspace_key: Option<&str>) -> Vec<McpServerStatus> {
        let mut statuses = Vec::new();

        let Some(workspace_key) = workspace_key else {
            return statuses;
        };

        if let Some(workspace) = self.workspaces.get(workspace_key) {
            for (name, client_entry) in workspace.value().servers.iter() {
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
                    name: name.clone(),
                    source: client_entry.source,
                    status: client_entry.status,
                    tools,
                    error: client_entry.error.clone(),
                });
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
        let workspace_root = self.canonicalize_workspace_root(workspace_root).await;
        let workspace_key = Self::workspace_key(&workspace_root);
        self.workspaces.remove(&workspace_key);
        // 重新初始化
        self.init_workspace_servers(&workspace_root, effective, workspace_settings)
            .await?;
        Ok(())
    }

    /// 获取工作区服务器数量
    pub fn workspace_server_count(&self, workspace_key: &str) -> usize {
        self.workspaces
            .get(workspace_key)
            .map(|w| w.value().servers.len())
            .unwrap_or(0)
    }
}

fn is_disabled(config: &McpServerConfig) -> bool {
    config.disabled
}
