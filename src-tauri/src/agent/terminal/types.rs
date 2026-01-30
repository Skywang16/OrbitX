use serde::{Deserialize, Serialize};

pub type TerminalId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TerminalExecutionMode {
    Blocking,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TerminalStatus {
    Initializing,
    Running,
    Completed { exit_code: Option<i32> },
    Failed { error: String },
    Aborted,
}

impl TerminalStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TerminalStatus::Completed { .. }
                | TerminalStatus::Failed { .. }
                | TerminalStatus::Aborted
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTerminal {
    pub id: TerminalId,
    pub command: String,
    pub pane_id: u32,
    pub mode: TerminalExecutionMode,
    pub status: TerminalStatus,
    pub session_id: i64,
    pub created_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub label: Option<String>,
}
