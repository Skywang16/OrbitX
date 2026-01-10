use std::collections::HashMap;

use crate::agent::error::{AgentError, AgentResult};
use std::sync::Arc;

use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, Message, MessageRole, ToolStatus};
use crate::llm::anthropic_types::{
    ContentBlock, ImageSource, MessageContent, MessageParam, MessageRole as AnthropicRole,
    ToolResultContent,
};

const TOOL_RESULT_CLEARED_TEXT: &str = "[tool result cleared]";

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
            .list_by_session_with_breakpoint(session_id)
            .await?;

        self.to_llm_messages(&messages).await
    }

    async fn to_llm_messages(&self, messages: &[Message]) -> AgentResult<Vec<MessageParam>> {
        let message_ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
        let outputs = self
            .persistence
            .tool_outputs()
            .list_by_message_ids(&message_ids)
            .await?;

        let mut output_map: HashMap<(i64, String), (String, Option<i64>)> = HashMap::new();
        for (message_id, block_id, output_content, compacted_at) in outputs {
            output_map.insert((message_id, block_id), (output_content, compacted_at));
        }

        let mut llm_messages = Vec::new();
        for message in messages {
            match message.role {
                MessageRole::User => {
                    llm_messages.push(MessageParam {
                        role: AnthropicRole::User,
                        content: map_user_blocks_to_content(&message.blocks)?,
                    });
                }
                MessageRole::Assistant => {
                    let (assistant_blocks, tool_results) =
                        map_assistant_blocks(&message, &output_map);

                    llm_messages.push(MessageParam {
                        role: AnthropicRole::Assistant,
                        content: MessageContent::Blocks(assistant_blocks),
                    });

                    if !tool_results.is_empty() {
                        llm_messages.push(MessageParam {
                            role: AnthropicRole::User,
                            content: MessageContent::Blocks(tool_results),
                        });
                    }
                }
            }
        }

        Ok(llm_messages)
    }
}

fn map_user_blocks_to_content(blocks: &[Block]) -> AgentResult<MessageContent> {
    let mut out = Vec::new();

    for block in blocks {
        match block {
            Block::UserText(b) => out.push(ContentBlock::Text {
                text: b.content.clone(),
                cache_control: None,
            }),
            Block::UserImage(b) => {
                let base64 = extract_base64_data(&b.data_url).ok_or_else(|| {
                    AgentError::Parse("Invalid user image dataUrl".to_string())
                })?;
                out.push(ContentBlock::Image {
                    source: ImageSource::Base64 {
                        media_type: b.mime_type.clone(),
                        data: base64,
                    },
                    cache_control: None,
                });
            }
            _ => {}
        }
    }

    if out.is_empty() {
        return Ok(MessageContent::Text(String::new()));
    }

    Ok(MessageContent::Blocks(out))
}

fn map_assistant_blocks(
    message: &Message,
    output_map: &HashMap<(i64, String), (String, Option<i64>)>,
) -> (Vec<ContentBlock>, Vec<ContentBlock>) {
    let mut assistant_blocks = Vec::new();
    let mut tool_results = Vec::new();

    for block in &message.blocks {
        match block {
            Block::Text(b) => {
                if !b.content.trim().is_empty() {
                    assistant_blocks.push(ContentBlock::Text {
                        text: b.content.clone(),
                        cache_control: None,
                    });
                }
            }
            Block::Tool(tool) => {
                assistant_blocks.push(ContentBlock::ToolUse {
                    id: tool.id.clone(),
                    name: tool.name.clone(),
                    input: tool.input.clone(),
                });

                if matches!(tool.status, ToolStatus::Running) {
                    continue;
                }

                let is_error = matches!(tool.status, ToolStatus::Error);
                let text = resolve_tool_output_text(message.id, tool, output_map);
                tool_results.push(ContentBlock::ToolResult {
                    tool_use_id: tool.id.clone(),
                    content: Some(ToolResultContent::Text(text)),
                    is_error: Some(is_error),
                });
            }
            Block::Error(b) => {
                assistant_blocks.push(ContentBlock::Text {
                    text: format!("[error] {}{}", b.message, b.details.as_deref().unwrap_or("")),
                    cache_control: None,
                });
            }
            _ => {}
        }
    }

    if assistant_blocks.is_empty() {
        assistant_blocks.push(ContentBlock::Text {
            text: String::new(),
            cache_control: None,
        });
    }

    (assistant_blocks, tool_results)
}

fn resolve_tool_output_text(
    message_id: i64,
    tool: &crate::agent::types::ToolBlock,
    output_map: &HashMap<(i64, String), (String, Option<i64>)>,
) -> String {
    let from_db = output_map.get(&(message_id, tool.id.clone()));

    let compacted = tool.compacted_at.is_some() || from_db.and_then(|(_, ts)| *ts).is_some();
    if compacted {
        return TOOL_RESULT_CLEARED_TEXT.to_string();
    }

    if let Some((content, _)) = from_db {
        return content.clone();
    }

    tool.output
        .as_ref()
        .map(|o| serde_json::to_string(&o.content).unwrap_or_default())
        .unwrap_or_default()
}

fn extract_base64_data(data_url: &str) -> Option<String> {
    let mut parts = data_url.splitn(2, ',');
    let _ = parts.next()?;
    parts.next().map(|s| s.to_string())
}
