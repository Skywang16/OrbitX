use serde::{Deserialize, Serialize};

/// Basic metadata describing the running agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
}
