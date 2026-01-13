use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionRules {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    },
    Sse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        disabled: bool,
    },
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        disabled: bool,
    },
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

/// AI设置 (全局和工作区共用结构)
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

/// 合并后的有效设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSettings {
    pub permissions: PermissionRules,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub rules_content: String,
    pub agent: AgentConfig,
}

impl EffectiveSettings {
    pub fn merge(global: &Settings, workspace: Option<&Settings>) -> Self {
        let default_workspace = Settings::default();
        let workspace = workspace.unwrap_or(&default_workspace);

        let permissions = PermissionRules {
            allow: union(&global.permissions.allow, &workspace.permissions.allow),
            deny: union(&global.permissions.deny, &workspace.permissions.deny),
            ask: union(&global.permissions.ask, &workspace.permissions.ask),
        };

        let mcp_servers = merge_maps(&global.mcp_servers, &workspace.mcp_servers);

        let rules_content = merge_rules_content(&global.rules.content, &workspace.rules.content);

        let agent = merge_agent(&global.agent, &workspace.agent);

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

fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut set = HashSet::<String>::with_capacity(a.len() + b.len());
    set.extend(a.iter().cloned());
    set.extend(b.iter().cloned());
    set.into_iter().collect()
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

fn merge_rules_content(global: &str, workspace: &str) -> String {
    let global = global.trim();
    let workspace = workspace.trim();

    match (global.is_empty(), workspace.is_empty()) {
        (true, true) => String::new(),
        (false, true) => global.to_string(),
        (true, false) => workspace.to_string(),
        (false, false) => format!("{global}\n\n{workspace}"),
    }
}

fn merge_agent(global: &AgentConfigPatch, workspace: &AgentConfigPatch) -> AgentConfig {
    let mut merged = AgentConfig::default();

    apply_agent_patch(&mut merged, global);
    apply_agent_patch(&mut merged, workspace);

    merged
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
