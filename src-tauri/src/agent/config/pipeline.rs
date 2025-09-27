use serde::{Deserialize, Serialize};

/// Execution pipeline configuration shared across agent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionConfig {
    pub max_iterations: u32,
    pub max_errors: u32,
    pub auto_retry_count: u32,
    pub state_save_interval: u64,
    pub verbose_logging: bool,
    pub tool_execution_timeout: u64,
    pub llm_call_timeout: u64,
}

impl Default for TaskExecutionConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_errors: 5,
            auto_retry_count: 3,
            state_save_interval: 30,
            verbose_logging: false,
            tool_execution_timeout: 300,
            llm_call_timeout: 60,
        }
    }
}
