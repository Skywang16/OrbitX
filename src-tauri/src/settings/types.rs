use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionRules {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}

/// MCP server configuration supporting multiple transport types.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        disabled: bool,
        #[serde(rename = "disabledTools", alias = "disabled_tools", default)]
        disabled_tools: Vec<String>,
    },
    Sse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        disabled: bool,
        #[serde(rename = "disabledTools", alias = "disabled_tools", default)]
        disabled_tools: Vec<String>,
    },
    #[serde(rename = "streamable_http")]
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        disabled: bool,
        #[serde(rename = "disabledTools", alias = "disabled_tools", default)]
        disabled_tools: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
struct StdioConfigFields {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    disabled: bool,
    #[serde(rename = "disabledTools", alias = "disabled_tools", default)]
    disabled_tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteConfigFields {
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    disabled: bool,
    #[serde(rename = "disabledTools", alias = "disabled_tools", default)]
    disabled_tools: Vec<String>,
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("MCP server config must be a JSON object"))?;

        let transport = object.get("type").and_then(Value::as_str);

        match transport {
            Some("stdio") => {
                let config: StdioConfigFields =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(McpServerConfig::Stdio {
                    command: config.command,
                    args: config.args,
                    env: config.env,
                    disabled: config.disabled,
                    disabled_tools: config.disabled_tools,
                })
            }
            Some("sse") => {
                let config: RemoteConfigFields =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(McpServerConfig::Sse {
                    url: config.url,
                    headers: config.headers,
                    disabled: config.disabled,
                    disabled_tools: config.disabled_tools,
                })
            }
            Some("streamable_http" | "streamable-http" | "http") => {
                let config: RemoteConfigFields =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(McpServerConfig::StreamableHttp {
                    url: config.url,
                    headers: config.headers,
                    disabled: config.disabled,
                    disabled_tools: config.disabled_tools,
                })
            }
            Some(other) => Err(D::Error::custom(format!(
                "Unsupported MCP transport type '{other}'. Expected one of: stdio, sse, streamable_http"
            ))),
            None if object.contains_key("command") => {
                // Compatibility with Claude Code / Codex-style stdio configs that omit `type`.
                let config: StdioConfigFields =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(McpServerConfig::Stdio {
                    command: config.command,
                    args: config.args,
                    env: config.env,
                    disabled: config.disabled,
                    disabled_tools: config.disabled_tools,
                })
            }
            None if object.contains_key("url") => {
                // Default URL-only configs to SSE for compatibility with common MCP examples.
                let config: RemoteConfigFields =
                    serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(McpServerConfig::Sse {
                    url: config.url,
                    headers: config.headers,
                    disabled: config.disabled,
                    disabled_tools: config.disabled_tools,
                })
            }
            None => Err(D::Error::custom(
                "MCP server config must include either `type`, `command`, or `url`",
            )),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesConfig {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub rules_file: Option<String>,
    #[serde(default = "default_rules_files")]
    pub rules_files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigPatch {
    #[serde(default)]
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub max_token_budget: Option<u64>,
    #[serde(default)]
    pub thinking_enabled: Option<bool>,
    #[serde(default)]
    pub auto_summary_threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub max_token_budget: u64,
    pub thinking_enabled: bool,
    pub auto_summary_threshold: f32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_token_budget: 200_000,
            thinking_enabled: true,
            auto_summary_threshold: 0.7,
        }
    }
}

/// AI settings (shared structure for global and workspace)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default)]
    pub permissions: PermissionRules,

    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    #[serde(default)]
    pub rules: RulesConfig,

    #[serde(default)]
    pub agent: AgentConfigPatch,
}

/// Merged effective settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSettings {
    pub permissions: PermissionRules,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub rules_content: String,
    pub agent: AgentConfig,
}

impl EffectiveSettings {
    /// Merge settings in priority order: global → workspace → local.
    /// - `workspace`: from `.orbitx/settings.json` (shared, committed)
    /// - `local`:     from `.orbitx/settings.local.json` (personal, git-ignored)
    ///
    /// Each layer's allow/deny/ask lists are concatenated in order (later layers have higher
    /// effective priority because deny always wins regardless of order, and allow is checked
    /// before ask).
    pub fn merge(
        global: &Settings,
        workspace: Option<&Settings>,
        local: Option<&Settings>,
    ) -> Self {
        let empty = Settings::default();
        let workspace = workspace.unwrap_or(&empty);
        let local = local.unwrap_or(&empty);

        let permissions = PermissionRules {
            allow: merge_vec3(
                &global.permissions.allow,
                &workspace.permissions.allow,
                &local.permissions.allow,
            ),
            deny: merge_vec3(
                &global.permissions.deny,
                &workspace.permissions.deny,
                &local.permissions.deny,
            ),
            ask: merge_vec3(
                &global.permissions.ask,
                &workspace.permissions.ask,
                &local.permissions.ask,
            ),
        };

        let mcp_servers = {
            let mut m = merge_maps(&global.mcp_servers, &workspace.mcp_servers);
            for (k, v) in &local.mcp_servers {
                m.insert(k.clone(), v.clone());
            }
            m
        };

        let rules_content = merge_rules_content3(
            &global.rules.content,
            &workspace.rules.content,
            &local.rules.content,
        );

        let mut agent = AgentConfig::default();
        apply_agent_patch(&mut agent, &global.agent);
        apply_agent_patch(&mut agent, &workspace.agent);
        apply_agent_patch(&mut agent, &local.agent);

        Self {
            permissions,
            mcp_servers,
            rules_content,
            agent,
        }
    }
}

fn default_rules_files() -> Vec<String> {
    vec!["CLAUDE.md", "AGENTS.md", ".cursorrules"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn merge_vec3(a: &[String], b: &[String], c: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(a.len() + b.len() + c.len());
    out.extend(a.iter().cloned());
    out.extend(b.iter().cloned());
    out.extend(c.iter().cloned());
    out
}

fn merge_maps<V: Clone>(
    global: &HashMap<String, V>,
    workspace: &HashMap<String, V>,
) -> HashMap<String, V> {
    let mut merged = global.clone();
    for (key, value) in workspace {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

fn merge_rules_content3(global: &str, workspace: &str, local: &str) -> String {
    [global.trim(), workspace.trim(), local.trim()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn apply_agent_patch(target: &mut AgentConfig, patch: &AgentConfigPatch) {
    if let Some(v) = patch.max_iterations {
        target.max_iterations = v;
    }
    if let Some(v) = patch.max_token_budget {
        target.max_token_budget = v;
    }
    if let Some(v) = patch.thinking_enabled {
        target.thinking_enabled = v;
    }
    if let Some(v) = patch.auto_summary_threshold {
        target.auto_summary_threshold = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_stdio_serde() {
        let json = r#"{"type":"stdio","command":"npx","args":["-y","test"],"disabled":false}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"stdio""#));
        assert!(serialized.contains(r#""command":"npx""#));
    }

    #[test]
    fn test_mcp_config_sse_serde() {
        let json = r#"{"type":"sse","url":"https://example.com","disabled":false}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"sse""#));
        assert!(serialized.contains(r#""url":"https://example.com""#));
    }

    #[test]
    fn test_mcp_config_streamable_http_serde() {
        let json = r#"{"type":"streamable_http","url":"https://example.com"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"streamable_http""#));
    }

    #[test]
    fn test_mcp_config_stdio_without_type_defaults_to_stdio() {
        let json = r#"{"command":"npx","args":["-y","test"],"disabled":true}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"stdio""#));
        assert!(serialized.contains(r#""command":"npx""#));
        assert!(serialized.contains(r#""disabled":true"#));
    }

    #[test]
    fn test_mcp_config_url_without_type_defaults_to_sse() {
        let json = r#"{"url":"https://example.com/sse"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"sse""#));
        assert!(serialized.contains(r#""url":"https://example.com/sse""#));
    }

    #[test]
    fn test_mcp_config_http_alias_maps_to_streamable_http() {
        let json = r#"{"type":"http","url":"https://example.com/mcp"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains(r#""type":"streamable_http""#));
        assert!(serialized.contains(r#""url":"https://example.com/mcp""#));
    }
}
