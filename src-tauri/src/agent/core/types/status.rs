use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTaskStatus {
    Created,
    Running,
    Paused,
    Completed,
    Error,
    Cancelled,
}

impl AgentTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

// ExecutionStatus persistence was removed in the new agent system.
