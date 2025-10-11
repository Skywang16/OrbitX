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

        // 调高阈值到 90%，因为 summarizer 已经在 85% 时处理过了
        // MessageCompactor 应该只作为最后一道防线
        if current_tokens < (context_window as f32 * 0.90) as u32 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        self.compact_messages(messages).await
    }

    pub async fn compact_messages(
        &self,
        messages: Vec<LLMMessage>,
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
    /// 保留关键信息摘要，避免 LLM 重复执行相同操作
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
                                    result,
                                } => {
                                    // 保留关键摘要信息，避免重复操作
                                    let cleared_result =
                                        create_tool_result_summary(tool_name, result);
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

/// 为工具结果创建简洁摘要，保留关键信息避免重复操作
fn create_tool_result_summary(tool_name: &str, result: &serde_json::Value) -> serde_json::Value {
    match tool_name {
        "read_file" => {
            // 保留文件路径和行数信息
            if let Some(path) = result.get("path").and_then(|v| v.as_str()) {
                let total_lines = result
                    .get("totalLines")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                return json!({
                    "status": "cleared",
                    "tool": tool_name,
                    "summary": format!("File '{}' was read ({} lines)", path, total_lines),
                });
            }
        }
        "list_files" => {
            // 保留目录路径和文件数量
            if let Some(path) = result.get("path").and_then(|v| v.as_str()) {
                let count = result.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                return json!({
                    "status": "cleared",
                    "tool": tool_name,
                    "summary": format!("Directory '{}' was listed ({} entries)", path, count),
                });
            }
        }
        "execute_command" => {
            // 保留命令和退出码
            if let Some(command) = result.get("command").and_then(|v| v.as_str()) {
                let exit_code = result
                    .get("exitCode")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);
                return json!({
                    "status": "cleared",
                    "tool": tool_name,
                    "summary": format!("Command '{}' executed (exit code: {})", command, exit_code),
                });
            }
        }
        _ => {}
    }

    // 默认情况：只保留工具名
    json!({
        "status": "cleared",
        "tool": tool_name,
        "summary": format!("{} was executed successfully", tool_name),
    })
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
