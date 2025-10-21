use std::sync::{Arc, LazyLock};

use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::llm::anthropic_types::{ContentBlock, MessageContent, MessageParam};

static TOKEN_ENCODER: LazyLock<Arc<CoreBPE>> = LazyLock::new(|| {
    cl100k_base()
        .expect("failed to initialise tokenizer")
        .into()
});

pub fn count_text_tokens(text: &str) -> usize {
    TOKEN_ENCODER.encode_with_special_tokens(text).len()
}

/// 统计 Anthropic 原生消息的 token 数
pub fn count_message_param_tokens(message: &MessageParam) -> usize {
    match &message.content {
        MessageContent::Text(text) => count_text_tokens(text),
        MessageContent::Blocks(blocks) => blocks.iter().map(count_block_tokens).sum(),
    }
}

fn count_block_tokens(block: &ContentBlock) -> usize {
    match block {
        ContentBlock::Text { text, .. } => count_text_tokens(text),
        ContentBlock::Image { source, .. } => {
            // 基于元数据粗略估算图片块开销
            // 不展开二进制，仅按描述字段长度估算
            let serialized = serde_json::json!({ "type": "image", "source": source });
            count_text_tokens(&serialized.to_string())
        }
        ContentBlock::ToolUse { id, name, input } => {
            let payload = serde_json::json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input,
            });
            count_text_tokens(&payload.to_string())
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            let payload = serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
                "is_error": is_error,
            });
            count_text_tokens(&payload.to_string())
        }
        ContentBlock::Thinking { thinking, .. } => count_text_tokens(thinking),
    }
}
