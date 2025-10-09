use serde::{Deserialize, Serialize};

use crate::agent::persistence::ExecutionStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl From<ExecutionStatus> for AgentTaskStatus {
    fn from(value: ExecutionStatus) -> Self {
        match value {
            ExecutionStatus::Running => Self::Running,
            ExecutionStatus::Completed => Self::Completed,
            ExecutionStatus::Error => Self::Error,
            ExecutionStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<&AgentTaskStatus> for ExecutionStatus {
    fn from(value: &AgentTaskStatus) -> Self {
        match value {
            AgentTaskStatus::Cancelled => ExecutionStatus::Cancelled,
            AgentTaskStatus::Completed => ExecutionStatus::Completed,
            AgentTaskStatus::Error => ExecutionStatus::Error,
            AgentTaskStatus::Created | AgentTaskStatus::Running | AgentTaskStatus::Paused => {
                ExecutionStatus::Running
            }
        }
    }
}
