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

/// Stores a single client and its metadata
struct ClientEntry {
    source: McpServerSource,
    status: McpConnectionStatus,
    error: Option<String>,
    client: Option<Arc<McpClient>>,
    disabled_tools: Vec<String>,
}

struct WorkspaceMcpState {
    servers: BTreeMap<String, ClientEntry>,
}

#[derive(Default)]
pub struct McpRegistry {
    /// Workspace-specific MCP servers
    workspaces: DashMap<Arc<str>, WorkspaceMcpState>,
}

impl McpRegistry {
    async fn canonicalize_workspace_root(&self, workspace_root: &Path) -> PathBuf {
        match tokio::fs::canonicalize(workspace_root).await {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!(
                    "Failed to canonicalize MCP workspace root '{}': {}",
                    workspace_root.display(),
                    err
                );
                workspace_root.to_path_buf()
            }
        }
    }

    fn workspace_key(workspace_root: &Path) -> Arc<str> {
        Arc::from(workspace_root.to_string_lossy().to_string())
    }

    fn normalize_workspace_key(workspace_key: &str) -> String {
        match std::fs::canonicalize(Path::new(workspace_key)) {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(err) => {
                tracing::warn!(
                    "Failed to canonicalize MCP workspace key '{}': {}",
                    workspace_key,
                    err
                );
                PathBuf::from(workspace_key).to_string_lossy().to_string()
            }
        }
    }

    /// Initialize workspace MCP servers (effective = merged result of global + workspace)
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
                            disabled_tools: get_disabled_tools(config),
                        },
                    );
                }
                Err(McpError::Disabled) => continue,
                Err(McpError::UnsupportedTransport) => {
                    servers.insert(
                        name.clone(),
                        ClientEntry {
                            source,
                            status: McpConnectionStatus::Error,
                            error: Some(
                                "Unsupported transport type (only stdio is supported)".to_string(),
                            ),
                            client: None,
                            disabled_tools: get_disabled_tools(config),
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!(target: "mcp", server = %name, error = %e, "Failed to init MCP server");
                    servers.insert(
                        name.clone(),
                        ClientEntry {
                            source,
                            status: McpConnectionStatus::Error,
                            error: Some(e.to_string()),
                            client: None,
                            disabled_tools: get_disabled_tools(config),
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

    /// Get all available tools for workspace (workspace_key = canonical workspace path string)
    pub fn get_tools_for_workspace(&self, workspace_key: &str) -> Vec<McpToolAdapter> {
        let mut out = Vec::new();
        let normalized = Self::normalize_workspace_key(workspace_key);

        if let Some(workspace) = self.workspaces.get(normalized.as_str()) {
            for entry in workspace.value().servers.values() {
                let Some(client) = entry.client.as_ref() else {
                    continue;
                };
                out.extend(
                    client
                        .tools()
                        .iter()
                        .filter(|tool| !entry.disabled_tools.iter().any(|name| name == &tool.name))
                        .cloned()
                        .map(|tool| McpToolAdapter::new(Arc::clone(client), tool)),
                );
            }
        } else if let Some(workspace) = self.workspaces.get(workspace_key) {
            for entry in workspace.value().servers.values() {
                let Some(client) = entry.client.as_ref() else {
                    continue;
                };
                out.extend(
                    client
                        .tools()
                        .iter()
                        .filter(|tool| !entry.disabled_tools.iter().any(|name| name == &tool.name))
                        .cloned()
                        .map(|tool| McpToolAdapter::new(Arc::clone(client), tool)),
                );
            }
        }

        out
    }

    /// Get all server statuses (for frontend display)
    pub fn get_servers_status(&self, workspace_key: Option<&str>) -> Vec<McpServerStatus> {
        let mut statuses = Vec::new();

        let Some(workspace_key) = workspace_key else {
            return statuses;
        };

        if let Some(workspace) = self.workspaces.get(workspace_key) {
            for (name, client_entry) in workspace.value().servers.iter() {
                let tools: Vec<McpToolInfo> = match client_entry.client.as_ref() {
                    Some(client) => client
                        .tools()
                        .iter()
                        .map(|tool| McpToolInfo {
                            name: tool.name.clone(),
                            description: if tool.description.is_empty() {
                                None
                            } else {
                                Some(tool.description.clone())
                            },
                        })
                        .collect(),
                    None => Vec::new(),
                };

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

    /// Reload workspace servers
    pub async fn reload_workspace_servers(
        &self,
        workspace_root: &Path,
        effective: &EffectiveSettings,
        workspace_settings: Option<&Settings>,
    ) -> McpResult<()> {
        let workspace_root = self.canonicalize_workspace_root(workspace_root).await;
        let workspace_key = Self::workspace_key(&workspace_root);
        self.workspaces.remove(&workspace_key);
        // Re-initialize
        self.init_workspace_servers(&workspace_root, effective, workspace_settings)
            .await?;
        Ok(())
    }
}

fn is_disabled(config: &McpServerConfig) -> bool {
    match config {
        McpServerConfig::Stdio { disabled, .. } => *disabled,
        McpServerConfig::Sse { disabled, .. } => *disabled,
        McpServerConfig::StreamableHttp { disabled, .. } => *disabled,
    }
}

fn get_disabled_tools(config: &McpServerConfig) -> Vec<String> {
    match config {
        McpServerConfig::Stdio { disabled_tools, .. } => disabled_tools.clone(),
        McpServerConfig::Sse { disabled_tools, .. } => disabled_tools.clone(),
        McpServerConfig::StreamableHttp { disabled_tools, .. } => disabled_tools.clone(),
    }
}
