use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::tools::ToolResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReactPhase {
    Reasoning,
    Action,
    Observation,
    Completion,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactThought {
    pub id: Uuid,
    pub iteration: usize,
    pub raw: String,
    pub normalized: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactAction {
    pub id: Uuid,
    pub iteration: usize,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub issued_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactObservation {
    pub id: Uuid,
    pub iteration: usize,
    pub tool_name: String,
    pub outcome: ToolResult,
    pub observed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactIteration {
    pub id: Uuid,
    pub index: usize,
    pub started_at: i64,
    pub status: ReactPhase,
    pub thought: Option<ReactThought>,
    pub action: Option<ReactAction>,
    pub observation: Option<ReactObservation>,
    pub response: Option<String>,
    pub finish_reason: Option<FinishReason>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReactRuntimeSnapshot {
    pub iterations: Vec<ReactIteration>,
    pub final_response: Option<String>,
    pub stop_reason: Option<FinishReasonOrTerminal>,
    pub aborted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReasonOrTerminal {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Abort,
    Error,
}

impl From<FinishReason> for FinishReasonOrTerminal {
    fn from(value: FinishReason) -> Self {
        match value {
            FinishReason::Stop => FinishReasonOrTerminal::Stop,
            FinishReason::Length => FinishReasonOrTerminal::Length,
            FinishReason::ToolCalls => FinishReasonOrTerminal::ToolCalls,
            FinishReason::ContentFilter => FinishReasonOrTerminal::ContentFilter,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactRuntimeConfig {
    pub max_iterations: u32,
    pub max_consecutive_errors: u32,
    pub max_idle_rounds: u32,
}
