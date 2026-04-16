/*!
 * Agent executor type definitions
 */

use serde::{Deserialize, Serialize};

/// Image attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachment {
    #[serde(rename = "type")]
    pub attachment_type: String,
    pub data_url: String,
    pub mime_type: String,
}

/// Agent run execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteRunParams {
    /// Belongs to workspace (absolute path/normalized)
    pub workspace_path: String,
    /// Thread ID under the workspace.
    pub thread_id: i64,
    pub user_prompt: String,
    pub model_id: String,
    /// Optional per-request agent type override (does not persist to session).
    #[serde(default)]
    pub agent_type: Option<String>,
    /// Optional command ID for slash commands (e.g., "code-review", "skill-creator")
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<ImageAttachment>>,
    /// Runtime system reminders to inject into LLM context (not persisted to UI messages).
    /// These are wrapped in <system-reminder> tags when sent to the LLM.
    #[serde(skip, default)]
    pub system_reminders: Vec<String>,
}

/// Agent run summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSummary {
    pub run_id: String,
    pub thread_id: i64,
    pub status: String,
    pub current_iteration: i32,
    pub error_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// File context status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContextStatus {
    pub workspace_path: String,
    pub file_count: usize,
    pub files: Vec<String>,
}
