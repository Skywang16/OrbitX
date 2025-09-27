use serde::{Deserialize, Serialize};

/// High level knobs for the desktop agent runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub platform: String,
    pub max_react_num: u32,
    pub max_react_idle_rounds: u32,
    pub max_react_error_streak: u32,
    pub max_tokens: u32,
    pub max_retry_num: u32,
    pub compress_threshold: u32,
    pub large_text_length: u32,
    pub file_text_max_length: u32,
    pub max_dialogue_img_file_num: u32,
    pub tool_result_multimodal: bool,
    pub expert_mode: bool,
    pub expert_mode_todo_loop_num: u32,
    pub max_agent_context_length: u32,
    pub enable_intelligent_compression: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "OrbitX".to_string(),
            platform: "mac".to_string(),
            max_react_num: 100,
            max_react_idle_rounds: 3,
            max_react_error_streak: 5,
            max_tokens: 8000,
            max_retry_num: 3,
            compress_threshold: 20,
            large_text_length: 15000,
            file_text_max_length: 50000,
            max_dialogue_img_file_num: 5,
            tool_result_multimodal: true,
            expert_mode: false,
            expert_mode_todo_loop_num: 10,
            max_agent_context_length: 200_000,
            enable_intelligent_compression: true,
        }
    }
}
