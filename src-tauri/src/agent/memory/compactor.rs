use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json;

use crate::agent::config::CompactionConfig;
use crate::agent::tokenizer::{count_message_tokens, count_text_tokens};
use crate::llm::service::LLMService;
use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart, LLMRequest};
use crate::storage::repositories::RepositoryManager;

pub struct MessageCompactor {
    config: CompactionConfig,
    llm_service: Arc<LLMService>,
}

impl MessageCompactor {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self {
            config: CompactionConfig::default(),
            llm_service: Arc::new(LLMService::new(repositories)),
        }
    }

    pub fn with_config(mut self, config: CompactionConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn compact_if_needed(
        &self,
        messages: Vec<LLMMessage>,
        model_id: &str,
    ) -> Result<CompactionResult> {
        if messages.len() < self.config.trigger_threshold {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        self.compact_messages(messages, model_id).await
    }

    pub async fn compact_messages(
        &self,
        messages: Vec<LLMMessage>,
        model_id: &str,
    ) -> Result<CompactionResult> {
        if messages.len() <= self.config.keep_recent_count + 1 {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        let (system_msg, middle, recent) = self.split_messages(&messages);
        if middle.is_empty() {
            return Ok(CompactionResult::NoCompaction(messages));
        }

        let original_tokens: u32 = middle
            .iter()
            .map(|msg| count_message_tokens(msg) as u32)
            .sum();

        let summary = self.summarize_messages(&middle, model_id).await?;
        let summary_tokens = count_text_tokens(&summary) as u32;

        let mut compacted = Vec::with_capacity(2 + recent.len());
        compacted.push(system_msg);
        compacted.push(LLMMessage {
            role: "system".to_string(),
            content: LLMMessageContent::Text(format!(
                "## Previous Conversation Summary\n\n{}\n\n_Note: Compressed from {} messages._",
                summary,
                middle.len()
            )),
        });
        compacted.extend(recent.into_iter());

        Ok(CompactionResult::Compacted {
            messages: compacted,
            tokens_saved: original_tokens.saturating_sub(summary_tokens),
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

    async fn summarize_messages(&self, messages: &[LLMMessage], model_id: &str) -> Result<String> {
        let prompt = self.build_summary_prompt(messages);
        let request = LLMRequest {
            model: model_id.to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text(
                        "You are an AI assistant summarizing previous conversation context. \
                        Extract key goals, actions, tool usage, files touched, and outstanding items.".to_string(),
                    ),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text(prompt),
                },
            ],
            temperature: Some(self.config.summary_temperature),
            max_tokens: Some(self.config.summary_max_tokens),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let response = self
            .llm_service
            .call(request)
            .await
            .map_err(|e| anyhow!("LLM summarization failed: {}", e))?;

        let content = response.content.trim();
        if content.is_empty() {
            return Err(anyhow!("Empty summary from LLM"));
        }

        Ok(content.to_string())
    }

    fn build_summary_prompt(&self, messages: &[LLMMessage]) -> String {
        let mut prompt = String::from("Summarize the following conversation segment:\n\n");
        for (idx, message) in messages.iter().enumerate() {
            let rendered = match &message.content {
                LLMMessageContent::Text(text) => text.clone(),
                LLMMessageContent::Parts(parts) => self.summarize_parts(parts),
            };
            prompt.push_str(&format!("{}. [{}] {}\n", idx + 1, message.role, rendered));
        }
        prompt.push_str(
            "\nProvide a concise summary focusing on:\n\
             - Main goals and decisions\n\
             - Files read/modified (paths only, NO file content)\n\
             - Tools used (name + outcome, NO raw output)\n\
             - Errors or unresolved issues\n",
        );
        prompt
    }

    fn summarize_parts(&self, parts: &[LLMMessagePart]) -> String {
        parts
            .iter()
            .map(|part| match part {
                LLMMessagePart::Text { text } => text.clone(),
                LLMMessagePart::ToolCall {
                    tool_name, args, ..
                } => {
                    let key_args = self.extract_key_args(tool_name, args);
                    format!("→ Call {}{}", tool_name, key_args)
                }
                LLMMessagePart::ToolResult {
                    tool_name, result, ..
                } => {
                    let file = result
                        .get("file_path")
                        .or_else(|| result.get("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let status = if result.get("error").is_some() {
                        "❌"
                    } else {
                        "✅"
                    };
                    if file.is_empty() {
                        format!("← {} {}", tool_name, status)
                    } else {
                        format!("← {} {} {}", tool_name, file, status)
                    }
                }
                LLMMessagePart::File { mime_type, data } => {
                    format!("[File: {} ({}B)]", mime_type, data.len())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extract_key_args(&self, tool_name: &str, args: &serde_json::Value) -> String {
        match tool_name {
            "read_file" | "edit_file" | "write_file" | "write_to_file" | "read_many_files"
            | "insert_content" | "apply_diff" | "unified_edit" => {
                if let Some(path) = args.get("path").or_else(|| args.get("file_path")) {
                    if let Some(p) = path.as_str() {
                        return format!(" {}", p);
                    }
                }
            }
            "shell" => {
                if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                    let short = if cmd.len() > 50 {
                        format!("{}...", &cmd[..50])
                    } else {
                        cmd.to_string()
                    };
                    return format!(" `{}`", short);
                }
            }
            _ => {}
        }
        String::new()
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
