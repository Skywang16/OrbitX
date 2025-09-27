use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub idle_rounds: u32,
    pub max_consecutive_errors: u32,
    pub max_iterations: u32,
    pub max_idle_rounds: u32,
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
            idle_rounds: 0,
            max_consecutive_errors: config.max_consecutive_errors,
            max_iterations: config.max_iterations,
            max_idle_rounds: config.max_idle_rounds,
            last_status_change: Utc::now().timestamp_millis(),
            status_change_reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskThresholds {
    pub max_consecutive_errors: u32,
    pub max_iterations: u32,
    pub max_idle_rounds: u32,
}

#[derive(Default, Clone)]
pub struct StateEventEmitter {
    listeners: Arc<Mutex<Vec<Arc<dyn Fn(&TaskStateEvent) + Send + Sync + 'static>>>>,
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

    pub fn on(&self, listener: Arc<dyn Fn(&TaskStateEvent) + Send + Sync + 'static>) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.push(listener);
        }
    }
}

#[derive(Clone)]
pub struct StateManager {
    state: TaskState,
    emitter: StateEventEmitter,
}

impl StateManager {
    pub fn new(initial_state: TaskState, emitter: StateEventEmitter) -> Self {
        Self {
            state: initial_state,
            emitter,
        }
    }

    pub fn state(&self) -> TaskState {
        self.state.clone()
    }

    pub fn update_task_status(&mut self, status: TaskStatus, reason: Option<String>) {
        let old_status = self.state.task_status.clone();
        self.state.task_status = status;
        self.state.last_status_change = Utc::now().timestamp_millis();
        self.state.status_change_reason = reason.clone();
        self.emitter.emit(TaskStateEvent {
            event_type: "task_status_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "oldStatus": old_status,
                "newStatus": self.state.task_status,
                "reason": reason,
                "timestamp": self.state.last_status_change,
            }),
        });
    }

    pub fn set_pause_status(&mut self, paused: bool, reason: Option<String>) {
        let old_paused = self.state.paused;
        self.state.paused = paused;
        self.state.pause_reason = reason.clone();
        self.emitter.emit(TaskStateEvent {
            event_type: "pause_status_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "oldPaused": old_paused,
                "newPaused": paused,
                "reason": reason,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn increment_error_count(&mut self) {
        self.state.consecutive_errors = self.state.consecutive_errors.saturating_add(1);
        self.emitter.emit(TaskStateEvent {
            event_type: "error_count_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "count": self.state.consecutive_errors,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn reset_error_count(&mut self) {
        self.state.consecutive_errors = 0;
        self.emitter.emit(TaskStateEvent {
            event_type: "error_count_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "count": 0,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn increment_iteration(&mut self) {
        self.state.iterations = self.state.iterations.saturating_add(1);
        self.emitter.emit(TaskStateEvent {
            event_type: "iteration_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "iterations": self.state.iterations,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn mark_idle_round(&mut self) {
        self.state.idle_rounds = self.state.idle_rounds.saturating_add(1);
        self.emitter.emit(TaskStateEvent {
            event_type: "idle_round_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "idleRounds": self.state.idle_rounds,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn reset_idle_rounds(&mut self) {
        self.state.idle_rounds = 0;
        self.emitter.emit(TaskStateEvent {
            event_type: "idle_round_changed".into(),
            payload: serde_json::json!({
                "taskId": self.state.task_id,
                "idleRounds": 0,
                "timestamp": Utc::now().timestamp_millis(),
            }),
        });
    }

    pub fn should_halt(&self) -> bool {
        self.state.consecutive_errors >= self.state.max_consecutive_errors
            || self.state.iterations >= self.state.max_iterations
            || self.state.idle_rounds >= self.state.max_idle_rounds
    }

    pub fn emitter(&self) -> &StateEventEmitter {
        &self.emitter
    }
}
