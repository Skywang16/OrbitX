use std::sync::Arc;

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, MessageRole};
use crate::llm::anthropic_types::{MessageContent, MessageParam, MessageRole as AnthropicRole};

pub struct SessionMessageLoader {
    persistence: Arc<AgentPersistence>,
}

impl SessionMessageLoader {
    pub fn new(persistence: Arc<AgentPersistence>) -> Self {
        Self { persistence }
    }

    pub async fn load_for_llm(&self, session_id: i64) -> AgentResult<Vec<MessageParam>> {
        let messages = self.persistence.messages().list_by_session(session_id).await?;
        let mut out = Vec::new();

        for msg in messages {
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
