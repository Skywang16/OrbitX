use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Thin task record used in prompt composition and runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub conversation_id: i64,
    pub user_prompt: String,
    pub xml: Option<String>,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Task node types derived from agent XML (<nodes><node>…</node></nodes>…)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskNode {
    pub text: String,
}

/// Detailed task description captured from planner output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    pub task_id: String,
    pub name: String,
    pub thought: String,
    pub description: String,
    pub nodes: Vec<TaskNode>,
    pub status: TaskDetailStatus,
    pub xml: String,
    pub modified: bool,
    pub task_prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskDetailStatus {
    Init,
    Running,
    Done,
    Error,
}

impl Default for TaskDetailStatus {
    fn default() -> Self {
        TaskDetailStatus::Init
    }
}

/// Planned task tree node used by TreePlanner outputs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlannedTaskNode {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlannedTask {
    pub name: Option<String>,
    pub thought: Option<String>,
    pub description: Option<String>,
    pub nodes: Option<Vec<PlannedTaskNode>>,
    pub subtasks: Option<Vec<PlannedTask>>,
}

pub type PlannedTaskTree = PlannedTask;

/// High level task status shared with front-end
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Created,
    Running,
    Paused,
    Completed,
    Error,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Created => write!(f, "created"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Paused => write!(f, "paused"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Error => write!(f, "error"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}
