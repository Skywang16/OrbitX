use chrono::Utc;

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, ToolStatus};

use super::config::CompactionConfig;

const TOOL_RESULT_CLEARED_TEXT: &str = "[tool result cleared]";

pub struct Pruner {
    persistence: std::sync::Arc<AgentPersistence>,
    config: CompactionConfig,
}

impl Pruner {
    pub fn new(persistence: std::sync::Arc<AgentPersistence>, config: CompactionConfig) -> Self {
        Self { persistence, config }
    }

    pub async fn prune_session(&self, session_id: i64) -> AgentResult<usize> {
        if !self.config.enabled || !self.config.auto_prune {
            return Ok(0);
        }

        let mut messages = self
            .persistence
            .messages()
            .list_by_session(session_id)
            .await?;

        if messages.len() <= self.config.keep_recent_messages {
            return Ok(0);
        }

        let keep_from = messages.len().saturating_sub(self.config.keep_recent_messages);
        let now = Utc::now();
        let compacted_at = now.timestamp();
        let mut tools_compacted = 0usize;

        for message in messages.iter_mut().take(keep_from) {
            let mut changed = false;

            for block in &mut message.blocks {
                let Block::Tool(tool) = block else { continue };
                if matches!(tool.status, ToolStatus::Running) {
                    continue;
                }

                if tool.output.is_none() {
                    continue;
                }

                if tool.compacted_at.is_some() {
                    continue;
                }

                if self
                    .config
                    .protected_tools
                    .iter()
                    .any(|name| name == &tool.name)
                {
                    continue;
                }

                self.persistence
                    .tool_outputs()
                    .mark_compacted(session_id, message.id, &tool.id, compacted_at)
                    .await?;

                tool.compacted_at = Some(now);
                if let Some(output) = &mut tool.output {
                    output.content = placeholder_for_output(&output.content);
                }

                tools_compacted += 1;
                changed = true;
            }

            if changed {
                self.persistence.messages().update(message).await?;
            }
        }

        Ok(tools_compacted)
    }
}

fn placeholder_for_output(existing: &serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(map) = existing {
        if map.contains_key("error") {
            return serde_json::json!({ "error": TOOL_RESULT_CLEARED_TEXT });
        }
        if map.contains_key("cancelled") {
            return serde_json::json!({ "cancelled": TOOL_RESULT_CLEARED_TEXT });
        }
        if map.contains_key("result") {
            return serde_json::json!({ "result": TOOL_RESULT_CLEARED_TEXT });
        }
    }
    serde_json::Value::String(TOOL_RESULT_CLEARED_TEXT.to_string())
}
