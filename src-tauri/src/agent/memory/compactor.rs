use serde_json::json;

use crate::agent::config::CompactionConfig;
use crate::agent::error::AgentResult;
use crate::agent::tokenizer::count_message_tokens;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart};

pub struct MessageCompactor {
    config: CompactionConfig,
}

impl MessageCompactor {
    pub fn new() -> Self {
        Self {
            config: CompactionConfig::default(),
        }
    }

    pub fn with_config(mut self, config: CompactionConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn compact_if_needed(
        &self,
        messages: Vec<LLMMessage>,
        _model_id: &str,
        context_window: u32,
    ) -> AgentResult<CompactionResult> {
        let current_tokens: u32 = messages
            .iter()
            .map(|msg| count_message_tokens(msg) as u32)
            .sum();

        // Only trigger when reaching 85% of context window
        if current_tokens < (context_window as f32 * 0.85) as u32 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        self.compact_messages(messages, current_tokens).await
    }

    pub async fn compact_messages(
        &self,
        messages: Vec<LLMMessage>,
        _current_tokens: u32,
    ) -> AgentResult<CompactionResult> {
        if messages.len() <= self.config.keep_recent_count + 1 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        let (system_msg, middle, recent) = self.split_messages(&messages);
        if middle.is_empty() {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        // Clear tool results from middle messages to save tokens
        let cleared_middle = self.clear_tool_results(&middle);

        let original_tokens: u32 = middle
            .iter()
            .map(|msg| count_message_tokens(msg) as u32)
            .sum();

        let cleared_tokens: u32 = cleared_middle
            .iter()
            .map(|msg| count_message_tokens(msg) as u32)
            .sum();

        let tokens_saved = original_tokens.saturating_sub(cleared_tokens);

        // If clearing didn't save enough (< 30%), drop older messages
        let cleared_count = cleared_middle.len();
        let final_middle = if tokens_saved < (original_tokens as f32 * 0.3) as u32 {
            // Keep only the more recent half of middle messages
            let keep_count = cleared_count / 2;
            cleared_middle
                .into_iter()
                .skip(cleared_count - keep_count)
                .collect()
        } else {
            cleared_middle
        };

        let final_tokens: u32 = final_middle
            .iter()
            .map(|msg| count_message_tokens(msg) as u32)
            .sum();

        let mut compacted = Vec::with_capacity(1 + final_middle.len() + recent.len());
        compacted.push(system_msg);
        compacted.extend(final_middle.into_iter());
        compacted.extend(recent.into_iter());

        Ok(CompactionResult::Compacted {
            messages: compacted,
            tokens_saved: original_tokens.saturating_sub(final_tokens),
            messages_summarized: middle.len(),
        })
    }

    fn split_messages(
        &self,
        messages: &[LLMMessage],
    ) -> (LLMMessage, Vec<LLMMessage>, Vec<LLMMessage>) {
        let system_msg = messages[0].clone();
        let keep_count = self
            .config
            .keep_recent_count
            .min(messages.len().saturating_sub(1));
        let split_point = messages.len() - keep_count;

        let middle = messages[1..split_point].to_vec();
        let recent = messages[split_point..].to_vec();

        (system_msg, middle, recent)
    }

    /// Clear tool results from messages to save tokens (Anthropic's "tool result clearing" strategy)
    fn clear_tool_results(&self, messages: &[LLMMessage]) -> Vec<LLMMessage> {
        messages
            .iter()
            .map(|msg| {
                let new_content = match &msg.content {
                    LLMMessageContent::Text(text) => LLMMessageContent::Text(text.clone()),
                    LLMMessageContent::Parts(parts) => {
                        let cleared_parts: Vec<LLMMessagePart> = parts
                            .iter()
                            .map(|part| match part {
                                LLMMessagePart::ToolResult {
                                    tool_call_id,
                                    tool_name,
                                    result: _,
                                } => {
                                    // Keep only metadata, clear the actual result content
                                    let cleared_result = json!({
                                        "status": "cleared",
                                        "original_tool": tool_name,
                                    });
                                    LLMMessagePart::ToolResult {
                                        tool_call_id: tool_call_id.clone(),
                                        tool_name: tool_name.clone(),
                                        result: cleared_result,
                                    }
                                }
                                other => other.clone(),
                            })
                            .collect();
                        LLMMessageContent::Parts(cleared_parts)
                    }
                };
                LLMMessage {
                    role: msg.role.clone(),
                    content: new_content,
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum CompactionResult {
    NoCompaction(Vec<LLMMessage>),
    Compacted {
        messages: Vec<LLMMessage>,
        tokens_saved: u32,
        messages_summarized: usize,
    },
}

impl CompactionResult {
    pub fn messages(self) -> Vec<LLMMessage> {
        match self {
            CompactionResult::NoCompaction(msgs) => msgs,
            CompactionResult::Compacted { messages, .. } => messages,
        }
    }

    pub fn was_compacted(&self) -> bool {
        matches!(self, CompactionResult::Compacted { .. })
    }
}
