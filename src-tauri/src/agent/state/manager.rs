use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
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

#[derive(Default, Clone)]
pub struct StateEventEmitter {
    listeners: Arc<Mutex<Vec<Box<dyn Fn(&TaskStateEvent) + Send + Sync + 'static>>>>,
}

#[derive(Debug, Clone)]
pub struct TaskStateEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
}

impl StateEventEmitter {
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn emit(&self, event: TaskStateEvent) {
        if let Ok(listeners) = self.listeners.lock() {
            for listener in listeners.iter() {
                listener(&event);
            }
        }
    }

    pub fn on(&self, listener: Box<dyn Fn(&TaskStateEvent) + Send + Sync + 'static>) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.push(listener);
        }
    }
}

pub struct StateManager {
    state: Arc<RwLock<TaskState>>,
    emitter: StateEventEmitter,
}

impl StateManager {
    pub fn new(initial_state: TaskState, emitter: StateEventEmitter) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            emitter,
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
        let event = {
            let mut state = self.state.write().await;
            let old_status = state.task_status;
            let task_id = state.task_id.clone();
            state.task_status = status;
            state.last_status_change = timestamp;
            state.status_change_reason = reason.clone();
            TaskStateEvent {
                event_type: "task_status_changed".into(),
                payload: serde_json::json!({
                    "taskId": task_id,
                    "oldStatus": old_status,
                    "newStatus": state.task_status,
                    "reason": reason,
                    "timestamp": timestamp,
                }),
            }
        };
        self.emitter.emit(event);
    }

    pub async fn set_pause_status(&self, paused: bool, reason: Option<String>) {
        let timestamp = Utc::now().timestamp_millis();
        let event = {
            let mut state = self.state.write().await;
            let old_paused = state.paused;
            let task_id = state.task_id.clone();
            state.paused = paused;
            state.pause_reason = reason.clone();
            TaskStateEvent {
                event_type: "pause_status_changed".into(),
                payload: serde_json::json!({
                    "taskId": task_id,
                    "oldPaused": old_paused,
                    "newPaused": paused,
                    "reason": reason,
                    "timestamp": timestamp,
                }),
            }
        };
        self.emitter.emit(event);
    }

    pub async fn increment_error_count(&self) {
        let timestamp = Utc::now().timestamp_millis();
        let event = {
            let mut state = self.state.write().await;
            state.consecutive_errors = state.consecutive_errors.saturating_add(1);
            let task_id = state.task_id.clone();
            let count = state.consecutive_errors;
            TaskStateEvent {
                event_type: "error_count_changed".into(),
                payload: serde_json::json!({
                    "taskId": task_id,
                    "count": count,
                    "timestamp": timestamp,
                }),
            }
        };
        self.emitter.emit(event);
    }

    pub async fn reset_error_count(&self) {
        let timestamp = Utc::now().timestamp_millis();
        let event = {
            let mut state = self.state.write().await;
            state.consecutive_errors = 0;
            let task_id = state.task_id.clone();
            TaskStateEvent {
                event_type: "error_count_changed".into(),
                payload: serde_json::json!({
                    "taskId": task_id,
                    "count": 0,
                    "timestamp": timestamp,
                }),
            }
        };
        self.emitter.emit(event);
    }

    pub async fn increment_iteration(&self) {
        let timestamp = Utc::now().timestamp_millis();
        let event = {
            let mut state = self.state.write().await;
            state.iterations = state.iterations.saturating_add(1);
            let task_id = state.task_id.clone();
            let iterations = state.iterations;
            TaskStateEvent {
                event_type: "iteration_changed".into(),
                payload: serde_json::json!({
                    "taskId": task_id,
                    "iterations": iterations,
                    "timestamp": timestamp,
                }),
            }
        };
        self.emitter.emit(event);
    }

    pub async fn should_halt(&self) -> bool {
        let state = self.state.read().await;
        state.consecutive_errors >= state.max_consecutive_errors
            || state.iterations >= state.max_iterations
    }

    pub fn emitter(&self) -> &StateEventEmitter {
        &self.emitter
    }
}
