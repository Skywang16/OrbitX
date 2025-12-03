use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;
use tracing::warn;

use crate::agent::config::ContextBuilderConfig;
use crate::agent::context::FileContextTracker;
use crate::agent::react::types::ReactIteration;
use crate::agent::tools::ToolResult;
use crate::llm::anthropic_types::{MessageContent, MessageParam};

#[derive(Clone)]
pub struct ContextBuilder {
    file_tracker: Arc<FileContextTracker>,
    config: ContextBuilderConfig,
}

impl ContextBuilder {
    pub fn new(file_tracker: Arc<FileContextTracker>) -> Self {
        Self {
            file_tracker,
            config: ContextBuilderConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ContextBuilderConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn build_file_context_message(
        &self,
        recent_iterations: &[ReactIteration],
    ) -> Option<MessageParam> {
        let mentioned = self.extract_mentioned_files(recent_iterations);
        if mentioned.is_empty() {
            return None;
        }

        let active_files = match self.file_tracker.get_active_files().await {
            Ok(files) => files,
            Err(err) => {
                warn!("failed to load active files: {}", err);
                return None;
            }
        };

        let stale_files = match self.file_tracker.get_stale_files().await {
            Ok(files) => files,
            Err(err) => {
                warn!("failed to load stale files: {}", err);
                return None;
            }
        };

        let mentioned_set: HashSet<_> = mentioned.iter().cloned().collect();
        let mut relevant_active: Vec<_> = active_files
            .into_iter()
            .filter(|entry| mentioned_set.contains(&entry.file_path))
            .collect();
        let mut relevant_stale: Vec<_> = stale_files
            .into_iter()
            .filter(|entry| mentioned_set.contains(&entry.file_path))
            .collect();

        if relevant_active.is_empty() && relevant_stale.is_empty() {
            return None;
        }

        relevant_active.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        relevant_stale.sort_by(|a, b| a.file_path.cmp(&b.file_path));

        let mut content = String::new();
        content.push_str("Files referenced in recent work:\n\n");

        if !relevant_active.is_empty() {
            content.push_str("Active files:\n");
            for entry in &relevant_active {
                content.push_str("- ");
                content.push_str(&entry.file_path);
                if let Some(ts) = entry.agent_read_timestamp {
                    content.push_str(" (read ");
                    content.push_str(&format_elapsed(ts));
                    content.push(')');
                }
                if let Some(ts) = entry.agent_edit_timestamp {
                    content.push_str(" (edited ");
                    content.push_str(&format_elapsed(ts));
                    content.push(')');
                }
                content.push('\n');
            }
            content.push('\n');
        }

        if self.config.include_stale_hints && !relevant_stale.is_empty() {
            content.push_str("Stale files:\n");
            for entry in &relevant_stale {
                content.push_str("- ");
                content.push_str(&entry.file_path);
                if let Some(ts) = entry.user_edit_timestamp {
                    content.push_str(" (user updated ");
                    content.push_str(&format_elapsed(ts));
                    content.push(')');
                }
                content.push_str(" -> re-read with read_file\n");
            }
            content.push('\n');
        }

        let note = "Use read_file before editing to load contents.";
        content.push_str(note);

        if content.len() > self.config.max_file_context_chars {
            let limit = self.config.max_file_context_chars.saturating_sub(3);
            content.truncate(limit);
            content.push_str("...");
        }

        Some(MessageParam {
            role: crate::llm::anthropic_types::MessageRole::User,
            content: MessageContent::Text(content),
        })
    }

    fn extract_mentioned_files(&self, iterations: &[ReactIteration]) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut ordered = Vec::new();
        for iter in iterations.iter().rev().take(self.config.recent_file_window) {
            if let Some(action) = &iter.action {
                if let Some(path) = self.extract_file_from_tool_args(&action.arguments) {
                    if seen.insert(path.clone()) {
                        ordered.push(path);
                    }
                }
            }
            if let Some(observation) = &iter.observation {
                if let Some(path) =
                    self.extract_file_from_tool_result(&observation.tool_name, &observation.outcome)
                {
                    if seen.insert(path.clone()) {
                        ordered.push(path);
                    }
                }
            }
        }
        ordered
    }

    fn extract_file_from_tool_args(&self, args: &Value) -> Option<String> {
        for key in [
            "path",
            "file_path",
            "target",
            "workspace_path",
            "source_path",
        ] {
            if let Some(value) = args.get(key) {
                if let Some(path) = value.as_str() {
                    return Some(path.to_string());
                }
            }
        }
        if let Some(value) = args.get("paths").and_then(|v| v.as_array()) {
            for item in value {
                if let Some(path) = item.as_str() {
                    return Some(path.to_string());
                }
            }
        }
        None
    }

    fn extract_file_from_tool_result(
        &self,
        _tool_name: &str,
        result: &ToolResult,
    ) -> Option<String> {
        if let Some(ext) = &result.ext_info {
            for key in ["file_path", "path", "target", "workspace_path"] {
                if let Some(value) = ext.get(key).and_then(|v| v.as_str()) {
                    return Some(value.to_string());
                }
            }
        }
        // File 变体已被移除,现在文件路径都在 ext_info 中
        None
    }
}

fn format_elapsed(ts: chrono::DateTime<Utc>) -> String {
    let delta = Utc::now() - ts;
    if delta < Duration::seconds(60) {
        return format!("{}s ago", delta.num_seconds().max(0));
    }
    if delta < Duration::minutes(60) {
        return format!("{}m ago", delta.num_minutes());
    }
    if delta < Duration::hours(24) {
        return format!("{}h ago", delta.num_hours());
    }
    format!("{}d ago", delta.num_days())
}
