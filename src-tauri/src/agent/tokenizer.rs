use std::sync::{Arc, LazyLock};

use serde_json::Value;
use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart};

static TOKEN_ENCODER: LazyLock<Arc<CoreBPE>> = LazyLock::new(|| {
    cl100k_base()
        .expect("failed to initialise tokenizer")
        .into()
});

pub fn count_text_tokens(text: &str) -> usize {
    TOKEN_ENCODER.encode_with_special_tokens(text).len()
}

pub fn count_message_tokens(message: &LLMMessage) -> usize {
    match &message.content {
        LLMMessageContent::Text(text) => count_text_tokens(text),
        LLMMessageContent::Parts(parts) => parts.iter().map(count_part_tokens).sum(),
    }
}

fn count_part_tokens(part: &LLMMessagePart) -> usize {
    match part {
        LLMMessagePart::Text { text } => count_text_tokens(text),
        LLMMessagePart::File { mime_type, data } => {
            let truncated = &data[..data.len().min(1024)];
            let payload = serde_json::json!({
                "type": "file",
                "mimeType": mime_type,
                "data": truncated,
            });
            count_text_tokens(&payload.to_string())
        }
        LLMMessagePart::ToolCall {
            tool_call_id,
            tool_name,
            args,
        } => count_structured_tokens("tool-call", tool_call_id, tool_name, args),
        LLMMessagePart::ToolResult {
            tool_call_id,
            tool_name,
            result,
        } => count_structured_tokens("tool-result", tool_call_id, tool_name, result),
    }
}

fn count_structured_tokens(
    part_type: &str,
    call_id: &str,
    tool_name: &str,
    payload: &Value,
) -> usize {
    let serialized = serde_json::json!({
        "type": part_type,
        "toolCallId": call_id,
        "toolName": tool_name,
        "payload": payload,
    })
    .to_string();
    count_text_tokens(&serialized)
}
