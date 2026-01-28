use serde::{Deserialize, Serialize};

use crate::agent::permissions::ToolFilter;

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
    /// Tool filter for agent capability boundaries (whitelist/blacklist).
    /// This is separate from Settings permissions (allow/deny/ask).
    pub tool_filter: ToolFilter,
    #[serde(default)]
    pub skills: Vec<String>,
    pub max_steps: Option<u32>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub color: Option<String>,
    pub hidden: bool,
    pub source_path: Option<String>,
    pub is_builtin: bool,
}
