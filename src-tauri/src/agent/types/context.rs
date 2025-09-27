use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workspace or execution context passed into prompt builders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub working_directory: Option<String>,
    pub workspace_info: Option<serde_json::Value>,
    pub environment_vars: HashMap<String, String>,
    pub additional_context: HashMap<String, serde_json::Value>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            working_directory: None,
            workspace_info: None,
            environment_vars: HashMap::new(),
            additional_context: HashMap::new(),
        }
    }
}
