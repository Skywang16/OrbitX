use serde::{Deserialize, Serialize};

/// Execution pipeline configuration shared across agent tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaskExecutionConfig {
    pub max_iterations: u32,
    pub max_errors: u32,
}

impl Default for TaskExecutionConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_errors: 5,
        }
    }
}
