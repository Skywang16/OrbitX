//! OpenAI Format Converter
//!
//!
//! ## Core Conversion Logic
//!
//! ### Message Role Mapping
//! - Anthropic `user` → OpenAI `user`
//! - Anthropic `assistant` → OpenAI `assistant`
//!
//! ### Tool Call Mapping
//! - Anthropic `tool_use` (sent by assistant) → OpenAI `tool_calls` array
//! - Anthropic `tool_result` (returned by user) → OpenAI `role: "tool"` message
//!
//! ### Special Handling
//! 1. **tool_result must immediately follow assistant.tool_calls**
//! 2. OpenAI does not support rich content (images) in tool_result, need to convert to text prompts
//! 3. System prompt is inserted as the first message

use crate::llm::anthropic_types::*;
use serde_json::{json, Value as JsonValue};

// ============================================================
// Main Conversion Functions
// ============================================================

/// Convert Anthropic messages to OpenAI format
///
/// Corresponds to TypeScript: `convertToOpenAiMessages()`
///
/// # Example
///
/// ```rust
/// use orbitx::llm::anthropic_types::*;
/// use orbitx::llm::transform::openai::convert_to_openai_messages;
///
/// let messages = vec![
///     MessageParam::user("Hello!"),
///     MessageParam::assistant("Hi! How can I help?"),
/// ];
///
/// let openai_messages = convert_to_openai_messages(&messages);
/// ```
pub fn convert_to_openai_messages<'a, I>(anthropic_messages: I) -> Vec<JsonValue>
where
    I: IntoIterator<Item = &'a MessageParam>,
{
    let mut openai_messages = Vec::new();

    for msg in anthropic_messages {
        match &msg.content {
            MessageContent::Text(text) => {
                // Simple text message: convert directly
                openai_messages.push(json!({
                    "role": role_to_string(msg.role),
                    "content": text,
                }));
            }
            MessageContent::Blocks(blocks) => {
                // Structured content: handle by role
                match msg.role {
                    MessageRole::User => handle_user_message(blocks, &mut openai_messages),
                    MessageRole::Assistant => {
                        handle_assistant_message(blocks, &mut openai_messages)
                    }
                }
            }
        }
    }

    openai_messages
}

// ============================================================
// User Message Handling
// ============================================================

/// Handle user messages (may contain tool_result)
///
/// OpenAI requirements:
/// 1. tool_result as a separate `role: "tool"` message
/// 2. tool_result must immediately follow assistant's tool_calls
/// 3. Regular content (text, images) as user messages
fn handle_user_message(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    let mut tool_results = Vec::new();
    let mut non_tool_content = Vec::new();

    // Separate tool_result from other content
    for block in blocks {
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error: _,
            } => {
                let content_str = match content {
                    Some(ToolResultContent::Text(text)) => text.clone(),
                    Some(ToolResultContent::Blocks(blocks)) => {
                        // OpenAI does not support rich content in tool result
                        // Convert blocks to text, use placeholder for images
                        blocks
                            .iter()
                            .map(|b| match b {
                                ToolResultBlock::Text { text } => text.as_str(),
                                ToolResultBlock::Image { .. } => {
                                    "(see image in following user message)"
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                    None => String::new(),
                };

                tool_results.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_use_id,
                    "content": content_str,
                }));
            }
            ContentBlock::Text { text, .. } => {
                non_tool_content.push(json!({
                    "type": "text",
                    "text": text,
                }));
            }
            ContentBlock::Image { source, .. } => {
                non_tool_content.push(convert_image_to_openai(source));
            }
            ContentBlock::ToolUse { .. } => {
                // user cannot send tool_use
            }
            ContentBlock::Thinking { .. } => {
                // thinking blocks are ignored in user messages
            }
        }
    }

    // Add tool results first (must immediately follow assistant's tool_calls)
    output.extend(tool_results);

    // Then add regular content
    if !non_tool_content.is_empty() {
        output.push(json!({
            "role": "user",
            "content": non_tool_content,
        }));
    }
}

// ============================================================
// Assistant Message Handling
// ============================================================

/// Handle assistant messages (may contain tool_use and thinking/reasoning blocks).
///
/// ## Reasoning trace handling
///
/// Thinking blocks carry provider-specific metadata (`signature` for Anthropic,
/// `item_id` + `encrypted_content` for OpenAI).  Dropping or flattening them
/// into plain text causes significant performance regressions (up to 30% on
/// Codex models).  We therefore emit them as structured `item_reference` objects
/// (for Responses API) or keep signature-bearing `thinking` blocks (for
/// Anthropic) rather than mixing them into the text content.
///
/// For the Chat Completions API path (non-Responses), thinking text is still
/// appended to the content field as a safe fallback because that API has no
/// first-class reasoning representation.
/// Handle assistant messages (may contain tool_use and thinking/reasoning).
///
/// Reasoning traces carry provider metadata (`item_id` for OpenAI,
/// `signature` for Anthropic).  Dropping them into plain text causes up to 30%
/// performance regression on Codex models.  We emit `item_reference` objects
/// when metadata is available, falling back to text inclusion otherwise.
fn handle_assistant_message(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    let mut text_parts: Vec<&str> = Vec::new();
    let mut tool_calls = Vec::new();
    let mut reasoning_refs: Vec<JsonValue> = Vec::new();
    // Track whether any thinking block had structured metadata.  If not, we
    // fall back to including thinking text in the content field.
    let mut thinking_texts: Vec<&str> = Vec::new();
    let mut has_structured_reasoning = false;

    for block in blocks {
        match block {
            ContentBlock::Text { text, .. } => {
                text_parts.push(text.as_str());
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(json!({
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": input.to_string() },
                }));
            }
            ContentBlock::Thinking {
                thinking,
                reasoning_metadata,
                ..
            } => {
                // Try structured item_reference first (OpenAI Responses API).
                if let Some(item_id) = reasoning_metadata
                    .as_ref()
                    .and_then(|m| m.item_id.as_deref())
                {
                    reasoning_refs.push(json!({
                        "type": "item_reference",
                        "id": item_id
                    }));
                    has_structured_reasoning = true;
                }
                if !thinking.is_empty() {
                    thinking_texts.push(thinking.as_str());
                }
            }
            ContentBlock::Image { .. } | ContentBlock::ToolResult { .. } => {}
        }
    }

    // Emit item_reference objects before the assistant message.
    output.extend(reasoning_refs);

    // Fallback: if no structured metadata, include thinking as text so the
    // model retains access to its previous reasoning via Chat Completions.
    if !has_structured_reasoning {
        text_parts.extend(thinking_texts);
    }

    let mut msg = json!({ "role": "assistant" });

    if !text_parts.is_empty() {
        msg["content"] = json!(text_parts.join("\n"));
    } else if tool_calls.is_empty() {
        msg["content"] = json!("");
    }

    if !tool_calls.is_empty() {
        msg["tool_calls"] = json!(tool_calls);
    }

    output.push(msg);
}

// ============================================================
// Helper Conversion Functions
// ============================================================

/// Convert image to OpenAI format
fn convert_image_to_openai(source: &ImageSource) -> JsonValue {
    match source {
        ImageSource::Base64 { media_type, data } => {
            json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", media_type, data)
                }
            })
        }
        ImageSource::Url { url } => {
            json!({
                "type": "image_url",
                "image_url": { "url": url }
            })
        }
        ImageSource::FileId { .. } => {
            // OpenAI does not support file_id, return placeholder
            json!({
                "type": "text",
                "text": "(File content not supported in OpenAI format)"
            })
        }
    }
}

/// Convert role enum to string
fn role_to_string(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

// ============================================================
// Responses API Conversion (Anthropic → OpenAI Responses API)
// ============================================================

/// Convert Anthropic messages directly to OpenAI Responses API `input` format.
///
/// This is a direct one-step conversion: Anthropic types → Responses API.
/// Do NOT route through `convert_to_openai_messages` (Chat Completions format)
/// as an intermediate step — they are sibling conversions, not a pipeline.
///
/// ## Responses API item types emitted
/// - `{"role":"user","content":[{"type":"input_text",...}]}` — user text/image
/// - `{"type":"function_call_output","call_id":...,"output":...}` — tool result
/// - `{"type":"function_call","call_id":...,"name":...,"arguments":...}` — assistant tool call
/// - `{"role":"assistant","content":[{"type":"output_text",...}]}` — assistant text
/// - `{"type":"item_reference","id":...}` — reasoning trace reference
pub fn convert_to_responses_api_input(messages: &[MessageParam]) -> Vec<JsonValue> {
    let mut out = Vec::new();

    for msg in messages {
        match msg.role {
            MessageRole::User => responses_user_message(&msg.content, &mut out),
            MessageRole::Assistant => responses_assistant_message(&msg.content, &mut out),
        }
    }

    out
}

fn responses_user_message(content: &MessageContent, out: &mut Vec<JsonValue>) {
    match content {
        MessageContent::Text(text) => {
            out.push(json!({
                "type": "message",
                "role": "user",
                "content": [{ "type": "input_text", "text": text }],
            }));
        }
        MessageContent::Blocks(blocks) => {
            let mut content_items: Vec<JsonValue> = Vec::new();

            for block in blocks {
                match block {
                    ContentBlock::Text { text, .. } => {
                        content_items.push(json!({ "type": "input_text", "text": text }));
                    }
                    ContentBlock::Image { source, .. } => {
                        content_items.push(responses_image_item(source));
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        // Tool results are top-level items, not nested in user content.
                        // Flush any pending user content first to preserve ordering.
                        if !content_items.is_empty() {
                            out.push(json!({
                                "type": "message",
                                "role": "user",
                                "content": content_items,
                            }));
                            content_items = Vec::new();
                        }
                        let output = tool_result_to_string(content);
                        out.push(json!({
                            "type": "function_call_output",
                            "call_id": tool_use_id,
                            "output": output,
                        }));
                    }
                    // ToolUse and Thinking cannot appear in user messages
                    ContentBlock::ToolUse { .. } | ContentBlock::Thinking { .. } => {}
                }
            }

            if !content_items.is_empty() {
                out.push(json!({
                    "type": "message",
                    "role": "user",
                    "content": content_items,
                }));
            }
        }
    }
}

fn responses_assistant_message(content: &MessageContent, out: &mut Vec<JsonValue>) {
    match content {
        MessageContent::Text(text) => {
            out.push(json!({
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": text }],
            }));
        }
        MessageContent::Blocks(blocks) => {
            let mut text_parts: Vec<&str> = Vec::new();

            for block in blocks {
                match block {
                    ContentBlock::Text { text, .. } => {
                        text_parts.push(text.as_str());
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        // Flush accumulated text before the function_call item.
                        if !text_parts.is_empty() {
                            let joined = text_parts.join("\n");
                            out.push(json!({
                                "type": "message",
                                "role": "assistant",
                                "content": [{ "type": "output_text", "text": joined }],
                            }));
                            text_parts = Vec::new();
                        }
                        out.push(json!({
                            "type": "function_call",
                            "call_id": id,
                            "name": name,
                            "arguments": input.to_string(),
                        }));
                    }
                    ContentBlock::Thinking {
                        reasoning_metadata, ..
                    } => {
                        // Emit item_reference when we have the OpenAI item_id.
                        if let Some(item_id) = reasoning_metadata
                            .as_ref()
                            .and_then(|m| m.item_id.as_deref())
                        {
                            out.push(json!({ "type": "item_reference", "id": item_id }));
                        }
                        // No structured metadata → skip; the model will re-reason.
                    }
                    ContentBlock::Image { .. } | ContentBlock::ToolResult { .. } => {}
                }
            }

            if !text_parts.is_empty() {
                let joined = text_parts.join("\n");
                out.push(json!({
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": joined }],
                }));
            }
        }
    }
}

fn responses_image_item(source: &ImageSource) -> JsonValue {
    match source {
        ImageSource::Base64 { media_type, data } => json!({
            "type": "input_image",
            "image_url": format!("data:{};base64,{}", media_type, data),
        }),
        ImageSource::Url { url } => json!({
            "type": "input_image",
            "image_url": url,
        }),
        ImageSource::FileId { file_id } => json!({
            "type": "input_image",
            "file_id": file_id,
        }),
    }
}

fn tool_result_to_string(content: &Option<ToolResultContent>) -> String {
    match content {
        Some(ToolResultContent::Text(text)) => text.clone(),
        Some(ToolResultContent::Blocks(blocks)) => blocks
            .iter()
            .map(|b| match b {
                ToolResultBlock::Text { text } => text.as_str(),
                ToolResultBlock::Image { .. } => "(image not supported in tool result)",
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    }
}

// ============================================================
// Reverse Conversion (OpenAI → Anthropic)
// ============================================================

/// Convert OpenAI response to Anthropic Message format
///
/// Used to convert OpenAI streaming or complete responses back to unified format
pub fn convert_openai_response_to_anthropic(
    openai_response: &JsonValue,
) -> Result<Message, String> {
    let choice = &openai_response["choices"][0];
    let message = &choice["message"];

    let mut content_blocks = Vec::new();

    // Handle text content
    if let Some(text) = message["content"].as_str() {
        if !text.is_empty() {
            content_blocks.push(ContentBlock::Text {
                text: text.to_string(),
                cache_control: None,
            });
        }
    }

    // Handle tool_calls
    if let Some(tool_calls) = message["tool_calls"].as_array() {
        for tool_call in tool_calls {
            let id = tool_call["id"].as_str().ok_or("Missing tool call id")?;
            let name = tool_call["function"]["name"]
                .as_str()
                .ok_or("Missing function name")?;
            let args_str = tool_call["function"]["arguments"]
                .as_str()
                .ok_or("Missing arguments")?;
            let input: JsonValue = serde_json::from_str(args_str)
                .map_err(|err| format!("Invalid tool call arguments JSON: {err}"))?;

            content_blocks.push(ContentBlock::ToolUse {
                id: id.to_string(),
                name: name.to_string(),
                input,
            });
        }
    }

    // Construct Message
    let id = openai_response["id"]
        .as_str()
        .ok_or("Missing OpenAI response id")?;
    let model = openai_response["model"]
        .as_str()
        .ok_or("Missing OpenAI response model")?;
    let input_tokens = openai_response["usage"]["prompt_tokens"]
        .as_u64()
        .ok_or("Missing OpenAI prompt token usage")?;
    let output_tokens = openai_response["usage"]["completion_tokens"]
        .as_u64()
        .ok_or("Missing OpenAI completion token usage")?;

    Ok(Message {
        id: id.to_string(),
        message_type: "message".to_string(),
        role: MessageRole::Assistant,
        content: content_blocks,
        model: model.to_string(),
        stop_reason: match choice["finish_reason"].as_str() {
            Some("stop") => Some(StopReason::EndTurn),
            Some("length") => Some(StopReason::MaxTokens),
            Some("tool_calls") => Some(StopReason::ToolUse),
            _ => None,
        },
        stop_sequence: None,
        usage: Usage {
            input_tokens: input_tokens as u32,
            output_tokens: output_tokens as u32,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: openai_response["usage"]["prompt_tokens_details"]
                ["cached_tokens"]
                .as_u64()
                .map(|n| n as u32),
        },
    })
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text_messages() {
        let messages = vec![
            MessageParam::user("Hello!"),
            MessageParam::assistant("Hi! How can I help?"),
        ];

        let result = convert_to_openai_messages(&messages);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"], "Hello!");
        assert_eq!(result[1]["role"], "assistant");
        assert_eq!(result[1]["content"], "Hi! How can I help?");
    }

    #[test]
    fn test_user_message_with_image() {
        let messages = vec![MessageParam::user_blocks(vec![
            ContentBlock::text("What's in this image?"),
            ContentBlock::image_url("https://example.com/image.jpg"),
        ])];

        let result = convert_to_openai_messages(&messages);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["role"], "user");
        assert!(result[0]["content"].is_array());
        assert_eq!(result[0]["content"][0]["type"], "text");
        assert_eq!(result[0]["content"][1]["type"], "image_url");
    }

    #[test]
    fn test_assistant_with_tool_calls() {
        let messages = vec![MessageParam::assistant_blocks(vec![
            ContentBlock::text("I'll check the weather for you."),
            ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "San Francisco"}),
            },
        ])];

        let result = convert_to_openai_messages(&messages);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["role"], "assistant");
        assert_eq!(result[0]["content"], "I'll check the weather for you.");
        assert!(result[0]["tool_calls"].is_array());
        assert_eq!(result[0]["tool_calls"][0]["id"], "call_123");
        assert_eq!(
            result[0]["tool_calls"][0]["function"]["name"],
            "get_weather"
        );
    }

    #[test]
    fn test_tool_results_as_separate_messages() {
        let messages = vec![MessageParam::user_blocks(vec![
            ContentBlock::tool_result("call_123", "Temperature: 72°F, Sunny"),
            ContentBlock::text("Based on this, should I bring an umbrella?"),
        ])];

        let result = convert_to_openai_messages(&messages);

        // Should generate 2 messages: 1 tool message + 1 user message
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "tool");
        assert_eq!(result[0]["tool_call_id"], "call_123");
        assert_eq!(result[0]["content"], "Temperature: 72°F, Sunny");
        assert_eq!(result[1]["role"], "user");
    }

    #[test]
    fn test_complete_tool_use_cycle() {
        let messages = vec![
            MessageParam::user("What's the weather in SF?"),
            MessageParam::assistant_blocks(vec![ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "San Francisco"}),
            }]),
            MessageParam::user_blocks(vec![ContentBlock::tool_result("call_123", "72°F, Sunny")]),
        ];

        let result = convert_to_openai_messages(&messages);

        assert_eq!(result.len(), 3);
        // User question
        assert_eq!(result[0]["role"], "user");
        // Assistant tool call
        assert_eq!(result[1]["role"], "assistant");
        assert!(result[1]["tool_calls"].is_array());
        // Tool result
        assert_eq!(result[2]["role"], "tool");
        assert_eq!(result[2]["tool_call_id"], "call_123");
    }

    #[test]
    fn test_base64_image_conversion() {
        let source = ImageSource::Base64 {
            media_type: "image/jpeg".to_string(),
            data: "base64data".to_string(),
        };

        let result = convert_image_to_openai(&source);

        assert_eq!(result["type"], "image_url");
        assert_eq!(
            result["image_url"]["url"],
            "data:image/jpeg;base64,base64data"
        );
    }

    #[test]
    fn test_openai_response_conversion() {
        let openai_response = json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello there!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5
            }
        });

        let result = convert_openai_response_to_anthropic(&openai_response).unwrap();

        assert_eq!(result.id, "chatcmpl-123");
        assert_eq!(result.role, MessageRole::Assistant);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello there!"),
            _ => panic!("Expected text block"),
        }
        assert_eq!(result.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(result.usage.input_tokens, 10);
        assert_eq!(result.usage.output_tokens, 5);
    }

    #[test]
    fn test_responses_api_messages_include_type() {
        let messages = vec![MessageParam::user("Hello!"), MessageParam::assistant("Hi!")];

        let result = convert_to_responses_api_input(&messages);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "message");
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[1]["type"], "message");
        assert_eq!(result[1]["role"], "assistant");
    }

    #[test]
    fn test_responses_api_flushes_user_message_before_tool_output() {
        let messages = vec![MessageParam::user_blocks(vec![
            ContentBlock::text("before"),
            ContentBlock::tool_result("call_123", "done"),
            ContentBlock::text("after"),
        ])];

        let result = convert_to_responses_api_input(&messages);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["type"], "message");
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[1]["type"], "function_call_output");
        assert_eq!(result[2]["type"], "message");
        assert_eq!(result[2]["role"], "user");
    }
}
