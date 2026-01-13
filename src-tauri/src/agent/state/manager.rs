use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Init,
    Running,
    Paused,
    Done,
    Error,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub task_status: TaskStatus,
    pub paused: bool,
    pub pause_reason: Option<String>,
    pub consecutive_errors: u32,
    pub iterations: u32,
    pub max_consecutive_errors: u32,
    pub max_iterations: u32,
    pub last_status_change: i64,
    pub status_change_reason: Option<String>,
}

impl TaskState {
    pub fn new(task_id: impl Into<String>, config: TaskThresholds) -> Self {
        Self {
            task_id: task_id.into(),
            task_status: TaskStatus::Init,
            paused: false,
            pause_reason: None,
            consecutive_errors: 0,
            iterations: 0,
            max_consecutive_errors: config.max_consecutive_errors,
            max_iterations: config.max_iterations,
            last_status_change: Utc::now().timestamp_millis(),
            status_change_reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskThresholds {
    pub max_consecutive_errors: u32,
    pub max_iterations: u32,
}

pub struct StateManager {
    state: RwLock<TaskState>,
}

impl StateManager {
    pub fn new(initial_state: TaskState) -> Self {
        Self {
            state: RwLock::new(initial_state),
        }
    }

    pub async fn snapshot(&self) -> TaskState {
        self.state.read().await.clone()
    }

    pub async fn task_status(&self) -> TaskStatus {
        self.state.read().await.task_status
    }

    pub async fn update_task_status(&self, status: TaskStatus, reason: Option<String>) {
        let timestamp = Utc::now().timestamp_millis();
        let mut state = self.state.write().await;
        state.task_status = status;
        state.last_status_change = timestamp;
        state.status_change_reason = reason;
    }

    pub async fn set_pause_status(&self, paused: bool, reason: Option<String>) {
        let mut state = self.state.write().await;
        state.paused = paused;
        state.pause_reason = reason;
    }

    pub async fn increment_error_count(&self) {
        let mut state = self.state.write().await;
        state.consecutive_errors = state.consecutive_errors.saturating_add(1);
    }

    pub async fn reset_error_count(&self) {
        self.state.write().await.consecutive_errors = 0;
    }

    pub async fn increment_iteration(&self) {
        let mut state = self.state.write().await;
        state.iterations = state.iterations.saturating_add(1);
    }

    pub async fn should_halt(&self) -> bool {
        let state = self.state.read().await;
        state.consecutive_errors >= state.max_consecutive_errors
            || state.iterations >= state.max_iterations
    }
}
