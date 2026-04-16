use chrono::{DateTime, Utc};

use crate::agent::core::context::AgentToolCallResult;
use crate::agent::state::session::ThreadContext;
use crate::llm::anthropic_types::MessageParam;
use std::sync::Arc;

pub struct IterationContext {
    pub iteration_num: u32,
    pub started_at: DateTime<Utc>,
    session: Arc<ThreadContext>,
    current_messages: Vec<MessageParam>,
    pending_tools: Vec<(String, String, serde_json::Value)>,
    tool_results: Vec<AgentToolCallResult>,
    thinking: String,
    output: String,
    files_touched: Vec<String>,
}

impl IterationContext {
    pub fn new(iteration_num: u32, session: Arc<ThreadContext>) -> Self {
        Self {
            iteration_num,
            started_at: Utc::now(),
            session,
            current_messages: Vec::new(),
            pending_tools: Vec::new(),
            tool_results: Vec::new(),
            thinking: String::new(),
            output: String::new(),
            files_touched: Vec::new(),
        }
    }

    pub fn session(&self) -> Arc<ThreadContext> {
        Arc::clone(&self.session)
    }

    pub fn add_tool_call(&mut self, id: String, name: String, arguments: serde_json::Value) {
        self.pending_tools.push((id, name, arguments));
    }

    pub fn add_tool_result(&mut self, result: AgentToolCallResult) {
        self.tool_results.push(result);
    }

    pub fn append_thinking(&mut self, text: &str) {
        self.thinking.push_str(text);
    }

    pub fn append_output(&mut self, text: &str) {
        self.output.push_str(text);
    }

    pub fn messages(&self) -> &[MessageParam] {
        &self.current_messages
    }

    pub fn tool_results(&self) -> &[AgentToolCallResult] {
        &self.tool_results
    }

    pub fn pending_tools(&self) -> &[(String, String, serde_json::Value)] {
        &self.pending_tools
    }

    pub fn thinking(&self) -> &str {
        &self.thinking
    }

    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn finalize(self) -> IterationSnapshot {
        let tools_used = self
            .pending_tools
            .iter()
            .map(|(_, name, _)| name.clone())
            .collect();
        let had_errors = self
            .tool_results
            .iter()
            .any(|result| result.status != crate::agent::tools::ToolResultStatus::Success);
        IterationSnapshot {
            iteration: self.iteration_num,
            started_at: self.started_at,
            completed_at: Utc::now(),
            thinking: self.thinking,
            output: self.output,
            messages_count: self.current_messages.len(),
            tools_used,
            files_touched: self.files_touched,
            had_errors,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IterationSnapshot {
    pub iteration: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub thinking: String,
    pub output: String,
    pub messages_count: usize,
    pub tools_used: Vec<String>,
    pub files_touched: Vec<String>,
    pub had_errors: bool,
}

impl IterationSnapshot {
    pub fn summarize(&self) -> String {
        let duration = (self.completed_at - self.started_at).num_seconds();
        let mut summary = format!("Iteration #{} ({}s): ", self.iteration, duration);

        if !self.output.is_empty() {
            let preview = if self.output.len() > 120 {
                crate::agent::utils::truncate_with_ellipsis(&self.output, 120)
            } else {
                self.output.clone()
            };
            summary.push_str(&preview);
        } else if !self.tools_used.is_empty() {
            summary.push_str(&format!("Used tools: {}", self.tools_used.join(", ")));
        } else {
            summary.push_str("Thinking...");
        }

        if self.had_errors {
            summary.push_str(" ⚠️ errors occurred");
        }

        summary
    }
}
