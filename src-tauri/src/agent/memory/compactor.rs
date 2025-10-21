
use crate::agent::config::CompactionConfig;
use crate::agent::error::AgentResult;
use crate::agent::utils::tokenizer::count_message_param_tokens;
use crate::llm::anthropic_types::{ContentBlock, MessageContent, MessageParam, ToolResultContent as AnthropicToolResultContent};

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
        messages: Vec<MessageParam>,
        _system: Option<crate::llm::anthropic_types::SystemPrompt>,
        _model_id: &str,
        context_window: u32,
    ) -> AgentResult<CompactionResult> {
        let current_tokens: u32 = messages
            .iter()
            .map(|msg| count_message_param_tokens(msg) as u32)
            .fold(0u32, |acc, n| acc.saturating_add(n));

        if context_window == 0 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        // 调高阈值到 90%，MessageCompactor 作为最后一道防线
        if current_tokens < (context_window as f32 * 0.90) as u32 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        self.compact_messages(messages).await
    }

    pub async fn compact_messages(
        &self,
        messages: Vec<MessageParam>,
    ) -> AgentResult<CompactionResult> {
        if messages.len() <= self.config.keep_recent_count {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        let (middle, recent) = self.split_messages(&messages);
        if middle.is_empty() {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        // 清理中段的 ToolResult 内容，保留轻量摘要
        let cleared_middle = self.clear_tool_results(&middle);

        let original_tokens: u32 = middle
            .iter()
            .map(|msg| count_message_param_tokens(msg) as u32)
            .fold(0u32, |acc, n| acc.saturating_add(n));

        let cleared_tokens: u32 = cleared_middle
            .iter()
            .map(|msg| count_message_param_tokens(msg) as u32)
            .fold(0u32, |acc, n| acc.saturating_add(n));

        let tokens_saved = original_tokens.saturating_sub(cleared_tokens);

        // 若节省不足 30%，进一步丢弃较旧一半的中段消息
        let cleared_count = cleared_middle.len();
        let final_middle = if cleared_count > 0 && tokens_saved < (original_tokens as f32 * 0.3) as u32 {
            let keep_count = cleared_count / 2;
            cleared_middle
                .into_iter()
                .skip(cleared_count.saturating_sub(keep_count))
                .collect()
        } else {
            cleared_middle
        };

        let final_tokens: u32 = final_middle
            .iter()
            .map(|msg| count_message_param_tokens(msg) as u32)
            .fold(0u32, |acc, n| acc.saturating_add(n));

        let mut compacted = Vec::with_capacity(final_middle.len() + recent.len());
        compacted.extend(final_middle.into_iter());
        compacted.extend(recent.into_iter());

        Ok(CompactionResult::Compacted {
            messages: compacted,
            tokens_saved: original_tokens.saturating_sub(final_tokens),
            messages_summarized: middle.len(),
        })
    }

    // 按配置切分为中段和最近段（不再假定第一条是系统消息）
    fn split_messages(
        &self,
        messages: &[MessageParam],
    ) -> (Vec<MessageParam>, Vec<MessageParam>) {
        let keep_count = self.config.keep_recent_count.min(messages.len());
        let split_point = messages.len().saturating_sub(keep_count);
        let middle = messages[..split_point].to_vec();
        let recent = messages[split_point..].to_vec();
        (middle, recent)
    }

    /// 将 ToolResult 内容清理为轻量文本，降低 token 占用
    fn clear_tool_results(&self, messages: &[MessageParam]) -> Vec<MessageParam> {
        messages
            .iter()
            .map(|msg| {
                let new_content = match &msg.content {
                    MessageContent::Text(text) => MessageContent::Text(text.clone()),
                    MessageContent::Blocks(blocks) => {
                        let cleared_blocks: Vec<ContentBlock> = blocks
                            .iter()
                            .map(|b| match b {
                                ContentBlock::ToolResult { tool_use_id, is_error, .. } => ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.clone(),
                                    content: Some(AnthropicToolResultContent::Text("[tool result cleared]".to_string())),
                                    is_error: *is_error,
                                },
                                other => other.clone(),
                            })
                            .collect();
                        MessageContent::Blocks(cleared_blocks)
                    }
                };
                MessageParam { role: msg.role, content: new_content }
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum CompactionResult {
    NoCompaction(Vec<MessageParam>),
    Compacted {
        messages: Vec<MessageParam>,
        tokens_saved: u32,
        messages_summarized: usize,
    },
}

impl CompactionResult {
    pub fn messages(self) -> Vec<MessageParam> {
        match self {
            CompactionResult::NoCompaction(msgs) => msgs,
            CompactionResult::Compacted { messages, .. } => messages,
        }
    }

    pub fn was_compacted(&self) -> bool {
        matches!(self, CompactionResult::Compacted { .. })
    }
}
