use std::sync::Arc;

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, MessageRole, ToolStatus};
use crate::llm::anthropic_types::{MessageContent, MessageParam, MessageRole as AnthropicRole};

pub struct SessionMessageLoader {
    persistence: Arc<AgentPersistence>,
}

impl SessionMessageLoader {
    pub fn new(persistence: Arc<AgentPersistence>) -> Self {
        Self { persistence }
    }

    pub async fn load_for_llm(&self, session_id: i64) -> AgentResult<Vec<MessageParam>> {
        let messages = self
            .persistence
            .messages()
            .list_by_session(session_id)
            .await?;
        let mut out = Vec::new();

        let start_idx = messages
            .iter()
            .rposition(|m| {
                m.is_summary && matches!(m.status, crate::agent::types::MessageStatus::Completed)
            })
            .unwrap_or(0);

        for msg in messages.into_iter().skip(start_idx) {
            let Some(text) = extract_plain_text(&msg.blocks, &msg.role) else {
                continue;
            };

            let role = match msg.role {
                MessageRole::User => AnthropicRole::User,
                MessageRole::Assistant => AnthropicRole::Assistant,
            };

            out.push(MessageParam {
                role,
                content: MessageContent::Text(text),
            });
        }

        Ok(out)
    }
}

fn extract_plain_text(blocks: &[Block], role: &MessageRole) -> Option<String> {
    let mut parts = Vec::new();

    match role {
        MessageRole::User => {
            for block in blocks {
                if let Block::UserText(b) = block {
                    if !b.content.trim().is_empty() {
                        parts.push(b.content.trim().to_string());
                    }
                }
            }
        }
        MessageRole::Assistant => {
            for block in blocks {
                match block {
                    Block::Text(b) => {
                        if !b.content.trim().is_empty() {
                            parts.push(b.content.trim().to_string());
                        }
                    }
                    Block::Tool(b) => {
                        if matches!(
                            b.status,
                            ToolStatus::Completed | ToolStatus::Error | ToolStatus::Cancelled
                        ) {
                            if let Some(preview) = tool_block_preview(b) {
                                parts.push(preview);
                            }
                        }
                    }
                    Block::Subtask(b) => {
                        if let Some(summary) = &b.summary {
                            if !summary.trim().is_empty() {
                                parts.push(summary.trim().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let out = parts.join("\n");

    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn tool_block_preview(block: &crate::agent::types::ToolBlock) -> Option<String> {
    // Keep tool context small; we only need enough to prevent pointless re-runs.
    let mut out = String::new();
    out.push_str("Tool ");
    out.push_str(block.name.as_str());

    match block.status {
        ToolStatus::Completed => out.push_str(" completed."),
        ToolStatus::Error => out.push_str(" errored."),
        ToolStatus::Cancelled => out.push_str(" cancelled."),
        _ => {}
    }

    let Some(output) = block.output.as_ref() else {
        return Some(out);
    };

    let rendered = if let Some(s) = output.content.as_str() {
        s.to_string()
    } else {
        // Compact JSON to reduce token cost.
        serde_json::to_string(&output.content).unwrap_or_default()
    };

    let rendered = rendered.trim();
    if rendered.is_empty() {
        return Some(out);
    }

    const MAX: usize = 10000;
    out.push('\n');
    if rendered.len() > MAX {
        out.push_str(&rendered[..MAX]);
        out.push_str("\n\n[...content too long, truncated]");
    } else {
        out.push_str(rendered);
    }

    Some(out)
}
