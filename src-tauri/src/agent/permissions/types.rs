use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

impl Default for PermissionDecision {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone)]
pub struct ToolAction {
    pub tool: String,
    pub param_variants: Vec<String>,
    pub workspace_root: PathBuf,
}

impl ToolAction {
    pub fn new(
        tool: impl Into<String>,
        workspace_root: PathBuf,
        param_variants: Vec<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            param_variants,
            workspace_root,
        }
    }
}
