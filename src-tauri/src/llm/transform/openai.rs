//! OpenAI 格式转换器
//!
//!
//! ## 核心转换逻辑
//!
//! ### 消息角色映射
//! - Anthropic `user` → OpenAI `user`
//! - Anthropic `assistant` → OpenAI `assistant`
//!
//! ### 工具调用映射
//! - Anthropic `tool_use` (assistant发出) → OpenAI `tool_calls` 数组
//! - Anthropic `tool_result` (user返回) → OpenAI `role: "tool"` 消息
//!
//! ### 特殊处理
//! 1. **tool_result 必须紧跟 assistant.tool_calls**
//! 2. OpenAI 不支持 tool_result 中的富内容（图片），需要转为文本提示
//! 3. System prompt 作为第一条消息插入

use crate::llm::anthropic_types::*;
use serde_json::{json, Value as JsonValue};

// ============================================================
// 主转换函数
// ============================================================

/// 转换 Anthropic 消息为 OpenAI 格式
///
/// 对应 TypeScript: `convertToOpenAiMessages()`
///
/// # 示例
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
                // 简单文本消息：直接转换
                openai_messages.push(json!({
                    "role": role_to_string(msg.role),
                    "content": text,
                }));
            }
            MessageContent::Blocks(blocks) => {
                // 结构化内容：根据角色分别处理
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
// User 消息处理
// ============================================================

/// 处理 user 消息（可能包含 tool_result）
///
/// OpenAI 要求：
/// 1. tool_result 作为独立的 `role: "tool"` 消息
/// 2. tool_result 必须紧跟在 assistant 的 tool_calls 后面
/// 3. 普通内容（文本、图片）作为 user 消息
fn handle_user_message(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    let mut tool_results = Vec::new();
    let mut non_tool_content = Vec::new();

    // 分离 tool_result 和其他内容
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
                        // OpenAI 不支持 tool result 中的富内容
                        // 将块转为文本，图片用占位符表示
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
                // user 不能发送 tool_use
            }
            ContentBlock::Thinking { .. } => {
                // thinking 块在 user 消息中忽略
            }
        }
    }

    // 先添加 tool results（必须紧跟在 assistant 的 tool_calls 后面）
    output.extend(tool_results);

    // 再添加普通内容
    if !non_tool_content.is_empty() {
        output.push(json!({
            "role": "user",
            "content": non_tool_content,
        }));
    }
}

// ============================================================
// Assistant 消息处理
// ============================================================

/// 处理 assistant 消息（可能包含 tool_use）
///
/// OpenAI 格式：
/// ```json
/// {
///   "role": "assistant",
///   "content": "text content (optional)",
///   "tool_calls": [
///     {
///       "id": "call_xxx",
///       "type": "function",
///       "function": {
///         "name": "tool_name",
///         "arguments": "{\"key\": \"value\"}"
///       }
///     }
///   ]
/// }
/// ```
fn handle_assistant_message(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::Text { text, .. } => {
                text_parts.push(text.as_str());
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": input.to_string(),
                    },
                }));
            }
            ContentBlock::Thinking { thinking, .. } => {
                // Thinking 内容可以包含在 content 中
                text_parts.push(thinking.as_str());
            }
            ContentBlock::Image { .. } | ContentBlock::ToolResult { .. } => {
                // assistant 不能发送这些类型
            }
        }
    }

    let mut msg = json!({
        "role": "assistant",
    });

    // content 字段：有文本才添加
    if !text_parts.is_empty() {
        msg["content"] = json!(text_parts.join("\n"));
    } else if tool_calls.is_empty() {
        // OpenAI 要求：content 和 tool_calls 至少有一个
        msg["content"] = json!("");
    }

    // tool_calls 字段：有工具调用才添加
    if !tool_calls.is_empty() {
        msg["tool_calls"] = json!(tool_calls);
    }

    output.push(msg);
}

// ============================================================
// 辅助转换函数
// ============================================================

/// 转换图片为 OpenAI 格式
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
            // OpenAI 不支持 file_id，返回占位符
            json!({
                "type": "text",
                "text": "(File content not supported in OpenAI format)"
            })
        }
    }
}

/// 转换角色枚举为字符串
fn role_to_string(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

// ============================================================
// 反向转换（OpenAI → Anthropic）
// ============================================================

/// 转换 OpenAI 响应为 Anthropic Message 格式
///
/// 用于将 OpenAI 的流式响应或完整响应转换回统一格式
pub fn convert_openai_response_to_anthropic(
    openai_response: &JsonValue,
) -> Result<Message, String> {
    let choice = &openai_response["choices"][0];
    let message = &choice["message"];

    let mut content_blocks = Vec::new();

    // 处理文本内容
    if let Some(text) = message["content"].as_str() {
        if !text.is_empty() {
            content_blocks.push(ContentBlock::Text {
                text: text.to_string(),
                cache_control: None,
            });
        }
    }

    // 处理 tool_calls
    if let Some(tool_calls) = message["tool_calls"].as_array() {
        for tool_call in tool_calls {
            let id = tool_call["id"].as_str().ok_or("Missing tool call id")?;
            let name = tool_call["function"]["name"]
                .as_str()
                .ok_or("Missing function name")?;
            let args_str = tool_call["function"]["arguments"]
                .as_str()
                .ok_or("Missing arguments")?;
            let input: JsonValue = serde_json::from_str(args_str).unwrap_or(json!({}));

            content_blocks.push(ContentBlock::ToolUse {
                id: id.to_string(),
                name: name.to_string(),
                input,
            });
        }
    }

    // 构造 Message
    Ok(Message {
        id: openai_response["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        message_type: "message".to_string(),
        role: MessageRole::Assistant,
        content: content_blocks,
        model: openai_response["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        stop_reason: match choice["finish_reason"].as_str() {
            Some("stop") => Some(StopReason::EndTurn),
            Some("length") => Some(StopReason::MaxTokens),
            Some("tool_calls") => Some(StopReason::ToolUse),
            _ => None,
        },
        stop_sequence: None,
        usage: Usage {
            input_tokens: openai_response["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            output_tokens: openai_response["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: openai_response["usage"]["prompt_tokens_details"]
                ["cached_tokens"]
                .as_u64()
                .map(|n| n as u32),
        },
    })
}

// ============================================================
// 测试
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

        // 应该生成2条消息：1条 tool 消息 + 1条 user 消息
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
}
