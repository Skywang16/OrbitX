use serde::{Deserialize, Serialize};

use crate::agent::permissions::PermissionDecision;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Primary,
    Subagent,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub name: String,
    pub description: Option<String>,
    pub mode: AgentMode,
    pub system_prompt: String,
    pub permission: AgentPermissionConfig,
    pub max_steps: Option<u32>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub color: Option<String>,
    pub hidden: bool,
    pub source_path: Option<String>,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPermissionConfig {
    pub default: PermissionDecision,
    pub read: Option<PermissionDecision>,
    pub edit: Option<PermissionDecision>,
    pub shell: Option<PermissionDecision>,
    pub grep: Option<PermissionDecision>,
    pub list: Option<PermissionDecision>,
    pub web_fetch: Option<PermissionDecision>,
    pub task: Option<PermissionDecision>,
    pub semantic_search: Option<PermissionDecision>,
}

impl AgentPermissionConfig {
    pub fn builtin_coder() -> Self {
        Self {
            default: PermissionDecision::Allow,
            read: None,
            edit: None,
            shell: None,
            grep: None,
            list: None,
            web_fetch: None,
            task: None,
            semantic_search: None,
        }
    }

    pub fn builtin_plan() -> Self {
        Self {
            default: PermissionDecision::Allow,
            read: None,
            edit: Some(PermissionDecision::Deny),
            shell: Some(PermissionDecision::Deny),
            grep: None,
            list: None,
            web_fetch: None,
            task: Some(PermissionDecision::Allow),
            semantic_search: None,
        }
    }
}

