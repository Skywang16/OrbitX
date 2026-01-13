use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
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
