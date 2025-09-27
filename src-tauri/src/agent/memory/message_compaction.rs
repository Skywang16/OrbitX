use crate::llm::types::{LLMMessage, LLMMessageContent, LLMMessagePart};

/// Compress conversation messages within a character budget while always keeping the first system message.
/// Strategy:
/// - Keep the very first message (usually system)
/// - From the tail, keep messages until accumulated length stays within the budget
/// - Ensure at least the last message is kept
pub fn compress_messages(mut messages: Vec<LLMMessage>, budget_chars: usize) -> Vec<LLMMessage> {
    if messages.is_empty() {
        return messages;
    }

    // Extract and keep the first message (typically system)
    let first = messages.remove(0);

    fn msg_len(m: &LLMMessage) -> usize {
        match &m.content {
            LLMMessageContent::Text(s) => s.len(),
            LLMMessageContent::Parts(parts) => {
                let mut total = 0usize;
                for p in parts {
                    match p {
                        LLMMessagePart::Text { text } => total += text.len(),
                        LLMMessagePart::ToolCall { args, .. } => {
                            total += 32 + serde_json::to_string(args).unwrap_or_default().len();
                        }
                        LLMMessagePart::ToolResult { result, .. } => {
                            total += 32 + serde_json::to_string(result).unwrap_or_default().len();
                        }
                        LLMMessagePart::File { data, .. } => {
                            // For large files, count a fixed overhead + truncated data length
                            total += 128 + data.len().min(1024);
                        }
                    }
                }
                total
            }
        }
    }

    let mut kept: Vec<LLMMessage> = Vec::new();
    let mut remaining = budget_chars.saturating_sub(msg_len(&first));

    // From the tail, pick messages until budget is exhausted; ensure at least keep the last one
    for m in messages.into_iter().rev() {
        let len = msg_len(&m);
        if kept.is_empty() || len <= remaining {
            remaining = remaining.saturating_sub(len);
            kept.push(m);
        } else {
            break;
        }
    }
    kept.reverse();

    let mut result = Vec::with_capacity(1 + kept.len());
    result.push(first);
    result.extend(kept);
    result
}
