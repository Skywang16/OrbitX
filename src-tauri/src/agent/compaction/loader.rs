use std::sync::Arc;

use serde_json::Value as JsonValue;
use tracing::warn;

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::rollout::replay_rollout;
use crate::agent::types::{Block, MessageRole, MessageStatus, ToolStatus};
use crate::llm::anthropic_types::{
    ContentBlock, MessageContent, MessageParam, MessageRole as AnthropicRole, ToolResultContent,
};

pub struct ThreadMessageLoader {
    persistence: Arc<AgentPersistence>,
}

impl ThreadMessageLoader {
    pub fn new(persistence: Arc<AgentPersistence>) -> Self {
        Self { persistence }
    }

    /// Load persisted messages into Anthropic `MessageParam` format.
    /// Guarantees strict User/Assistant alternation with User first.
    pub async fn load_for_llm(&self, thread_id: i64) -> AgentResult<Vec<MessageParam>> {
        let messages = if let Some(thread) = self.persistence.threads().get(thread_id).await? {
            match replay_rollout(std::path::Path::new(&thread.rollout_path)).await {
                Ok(replayed) if !replayed.messages.is_empty() => replayed.messages,
                Ok(_) => {
                    self.persistence
                        .messages()
                        .list_by_thread(thread_id)
                        .await?
                }
                Err(err) => {
                    warn!(
                        "Failed to replay rollout for thread {} at {}: {}",
                        thread_id, thread.rollout_path, err
                    );
                    self.persistence
                        .messages()
                        .list_by_thread(thread_id)
                        .await?
                }
            }
        } else {
            self.persistence
                .messages()
                .list_by_thread(thread_id)
                .await?
        };
        Ok(Self::messages_to_llm(messages))
    }

    pub fn messages_to_llm(messages: Vec<crate::agent::types::Message>) -> Vec<MessageParam> {
        let mut out: Vec<MessageParam> = Vec::new();

        let start_idx = messages
            .iter()
            .rposition(|m| {
                m.is_summary && matches!(m.status, crate::agent::types::MessageStatus::Completed)
            })
            .unwrap_or(0);

        for msg in messages.into_iter().skip(start_idx) {
            match msg.role {
                MessageRole::User => {
                    let text = extract_user_text(&msg.blocks).unwrap_or_else(|| ".".to_string());
                    push_msg(&mut out, AnthropicRole::User, MessageContent::Text(text));
                }
                MessageRole::Assistant => {
                    if matches!(msg.status, MessageStatus::Error) {
                        continue;
                    }

                    let (assistant_blocks, tool_results) = build_assistant_blocks(&msg.blocks);

                    if assistant_blocks.is_empty() {
                        continue;
                    }

                    push_msg(
                        &mut out,
                        AnthropicRole::Assistant,
                        MessageContent::Blocks(assistant_blocks),
                    );

                    if !tool_results.is_empty() {
                        push_msg(
                            &mut out,
                            AnthropicRole::User,
                            MessageContent::Blocks(tool_results),
                        );
                    }
                }
            }
        }

        if out
            .first()
            .is_some_and(|m| m.role == AnthropicRole::Assistant)
        {
            out.insert(
                0,
                MessageParam {
                    role: AnthropicRole::User,
                    content: MessageContent::Text(".".to_string()),
                },
            );
        }

        out
    }
}

/// Push a message, merging into the previous one if same role.
fn push_msg(out: &mut Vec<MessageParam>, role: AnthropicRole, content: MessageContent) {
    if let Some(last) = out.last_mut() {
        if last.role == role {
            let existing =
                std::mem::replace(&mut last.content, MessageContent::Text(String::new()));
            let mut blocks = to_blocks(existing);
            blocks.extend(to_blocks(content));
            last.content = MessageContent::Blocks(blocks);
            return;
        }
    }
    out.push(MessageParam { role, content });
}

fn to_blocks(content: MessageContent) -> Vec<ContentBlock> {
    match content {
        MessageContent::Blocks(b) => b,
        MessageContent::Text(t) => vec![ContentBlock::Text {
            text: t,
            cache_control: None,
        }],
    }
}

fn extract_user_text(blocks: &[Block]) -> Option<String> {
    let mut parts = Vec::new();
    for block in blocks {
        if let Block::UserText(b) = block {
            if !b.content.trim().is_empty() {
                parts.push(b.content.trim().to_string());
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

/// Split assistant blocks into (assistant content, tool_result) pair.
fn build_assistant_blocks(blocks: &[Block]) -> (Vec<ContentBlock>, Vec<ContentBlock>) {
    let mut assistant_blocks: Vec<ContentBlock> = Vec::new();
    let mut tool_results: Vec<ContentBlock> = Vec::new();

    for block in blocks {
        match block {
            Block::Text(b) => {
                if !b.content.trim().is_empty() {
                    assistant_blocks.push(ContentBlock::Text {
                        text: b.content.clone(),
                        cache_control: None,
                    });
                }
            }
            Block::Tool(b) => {
                assistant_blocks.push(ContentBlock::ToolUse {
                    id: b.call_id.clone(),
                    name: b.name.clone(),
                    input: b.input.clone(),
                });

                match b.status {
                    ToolStatus::Completed => {
                        let result_text = tool_output_text(b);
                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: b.call_id.clone(),
                            content: Some(ToolResultContent::Text(result_text)),
                            is_error: Some(false),
                        });
                    }
                    ToolStatus::Error => {
                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: b.call_id.clone(),
                            content: Some(ToolResultContent::Text(tool_output_text(b))),
                            is_error: Some(true),
                        });
                    }
                    ToolStatus::Cancelled => {
                        let reason = b
                            .output
                            .as_ref()
                            .and_then(|o| o.cancel_reason.clone())
                            .unwrap_or_else(|| "Tool execution was cancelled".to_string());
                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: b.call_id.clone(),
                            content: Some(ToolResultContent::Text(reason)),
                            is_error: Some(true),
                        });
                    }
                    ToolStatus::Pending | ToolStatus::Running => {
                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: b.call_id.clone(),
                            content: Some(ToolResultContent::Text(
                                "[Tool execution was interrupted]".to_string(),
                            )),
                            is_error: Some(true),
                        });
                    }
                }
            }
            Block::Thinking(b) => {
                if !b.content.trim().is_empty() {
                    let signature = b.metadata.as_ref().and_then(|m| m.signature.clone());
                    assistant_blocks.push(ContentBlock::Thinking {
                        thinking: b.content.clone(),
                        signature,
                        reasoning_metadata: b.metadata.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    (assistant_blocks, tool_results)
}

fn tool_output_text(block: &crate::agent::types::ToolBlock) -> String {
    let Some(output) = block.output.as_ref() else {
        return "(no output)".to_string();
    };

    if block.compacted_at.is_some() {
        return "[Old tool result content cleared]".to_string();
    }

    let rendered = match &output.content {
        JsonValue::String(s) => s.clone(),
        other => match serde_json::to_string(other) {
            Ok(serialized) => serialized,
            Err(err) => {
                warn!(
                    tool_name = %block.name,
                    tool_call_id = %block.call_id,
                    "Failed to serialize tool output content: {}",
                    err
                );
                format!("[failed to serialize tool output: {err}]")
            }
        },
    };

    let rendered = rendered.trim().to_string();
    if rendered.is_empty() {
        return "(empty result)".to_string();
    }

    rendered
}
