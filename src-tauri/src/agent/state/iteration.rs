use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::agent::core::context::ToolCallResult;
use crate::agent::state::session::SessionContext;
use crate::llm::anthropic_types::MessageParam;

pub struct IterationContext {
    pub iteration_num: u32,
    pub started_at: DateTime<Utc>,
    session: Arc<SessionContext>,
    current_messages: Arc<RwLock<Vec<MessageParam>>>,
    pending_tools: Arc<RwLock<Vec<(String, String, serde_json::Value)>>>,
    tool_results: Arc<RwLock<Vec<ToolCallResult>>>,
    thinking_buffer: Arc<RwLock<String>>,
    output_buffer: Arc<RwLock<String>>,
    files_touched: Arc<RwLock<Vec<String>>>,
}

impl IterationContext {
    pub fn new(iteration_num: u32, session: Arc<SessionContext>) -> Self {
        Self {
            iteration_num,
            started_at: Utc::now(),
            session,
            current_messages: Arc::new(RwLock::new(Vec::new())),
            pending_tools: Arc::new(RwLock::new(Vec::new())),
            tool_results: Arc::new(RwLock::new(Vec::new())),
            thinking_buffer: Arc::new(RwLock::new(String::new())),
            output_buffer: Arc::new(RwLock::new(String::new())),
            files_touched: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn session(&self) -> Arc<SessionContext> {
        Arc::clone(&self.session)
    }

    pub async fn add_message(&self, message: MessageParam) {
        self.current_messages.write().await.push(message);
    }

    pub async fn add_tool_call(&self, id: String, name: String, arguments: serde_json::Value) {
        self.pending_tools.write().await.push((id, name, arguments));
    }

    pub async fn add_tool_result(&self, result: ToolCallResult) {
        self.tool_results.write().await.push(result);
    }

    pub async fn append_thinking(&self, text: &str) {
        self.thinking_buffer.write().await.push_str(text);
    }

    pub async fn append_output(&self, text: &str) {
        self.output_buffer.write().await.push_str(text);
    }

    pub async fn track_file(&self, path: String) {
        let mut guard = self.files_touched.write().await;
        if !guard.contains(&path) {
            guard.push(path);
        }
    }

    pub async fn messages(&self) -> Vec<MessageParam> {
        self.current_messages.read().await.clone()
    }

    pub async fn tool_results(&self) -> Vec<ToolCallResult> {
        self.tool_results.read().await.clone()
    }

    pub async fn pending_tools(&self) -> Vec<(String, String, serde_json::Value)> {
        self.pending_tools.read().await.clone()
    }

    pub async fn finalize(self) -> IterationSnapshot {
        let thinking = self.thinking_buffer.read().await.clone();
        let output = self.output_buffer.read().await.clone();
        let files_touched = self.files_touched.read().await.clone();
        let tools_used = self
            .pending_tools
            .read()
            .await
            .iter()
            .map(|(_, name, _)| name.clone())
            .collect::<Vec<_>>();

        IterationSnapshot {
            iteration: self.iteration_num,
            started_at: self.started_at,
            completed_at: Utc::now(),
            thinking,
            output,
            messages_count: self.current_messages.read().await.len(),
            tools_used,
            files_touched,
            had_errors: self
                .tool_results
                .read()
                .await
                .iter()
                .any(|result| result.is_error),
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
