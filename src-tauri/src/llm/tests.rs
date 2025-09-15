#[cfg(test)]
mod tests {
    use crate::llm::{
        LLMMessage, LLMMessageContent, LLMMessagePart, LLMProviderConfig, LLMProviderType,
        LLMRequest, LLMStreamChunk, LLMTool, LLMUsage,
    };

    #[test]
    fn test_provider_types() {
        let providers = vec![
            LLMProviderType::OpenAI,
            LLMProviderType::Anthropic,
            LLMProviderType::Gemini,
            LLMProviderType::Qwen,
            LLMProviderType::Custom,
        ];

        // 测试序列化
        for provider in providers {
            let json = serde_json::to_string(&provider).unwrap();
            let _deserialized: LLMProviderType = serde_json::from_str(&json).unwrap();
            // 基本检查
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_message_content_serialization() {
        // 测试文本内容
        let text_message = LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Text("Hello world".to_string()),
        };

        let json = serde_json::to_string(&text_message).unwrap();
        let deserialized: LLMMessage = serde_json::from_str(&json).unwrap();

        match deserialized.content {
            LLMMessageContent::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text content"),
        }

        // 测试多部分内容
        let parts_message = LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Parts(vec![
                LLMMessagePart::Text {
                    text: "Hello".to_string(),
                },
                LLMMessagePart::File {
                    mime_type: "image/png".to_string(),
                    data: "base64data".to_string(),
                },
            ]),
        };

        let json = serde_json::to_string(&parts_message).unwrap();
        let deserialized: LLMMessage = serde_json::from_str(&json).unwrap();

        match deserialized.content {
            LLMMessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    LLMMessagePart::Text { text } => assert_eq!(text, "Hello"),
                    _ => panic!("Expected text part"),
                }
                match &parts[1] {
                    LLMMessagePart::File { mime_type, data } => {
                        assert_eq!(mime_type, "image/png");
                        assert_eq!(data, "base64data");
                    }
                    _ => panic!("Expected file part"),
                }
            }
            _ => panic!("Expected parts content"),
        }
    }

    #[test]
    fn test_stream_chunk_serialization() {
        let chunks = vec![
            LLMStreamChunk::Delta {
                content: Some("Hello".to_string()),
                tool_calls: None,
            },
            LLMStreamChunk::Delta {
                content: Some(" world".to_string()),
                tool_calls: None,
            },
            LLMStreamChunk::Finish {
                finish_reason: "stop".to_string(),
                usage: None,
            },
            LLMStreamChunk::Finish {
                finish_reason: "stop".to_string(),
                usage: Some(LLMUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
            },
            LLMStreamChunk::Error {
                error: "Test error".to_string(),
            },
        ];

        for chunk in chunks {
            let json = serde_json::to_string(&chunk).unwrap();
            let deserialized: LLMStreamChunk = serde_json::from_str(&json).unwrap();

            // 基本检查序列化/反序列化正常工作
            match (&chunk, &deserialized) {
                (
                    LLMStreamChunk::Delta {
                        content: c1,
                        tool_calls: t1,
                    },
                    LLMStreamChunk::Delta {
                        content: c2,
                        tool_calls: t2,
                    },
                ) => {
                    assert_eq!(c1, c2);
                    assert_eq!(t1, t2);
                }
                (
                    LLMStreamChunk::Finish {
                        finish_reason: r1,
                        usage: u1,
                    },
                    LLMStreamChunk::Finish {
                        finish_reason: r2,
                        usage: u2,
                    },
                ) => {
                    assert_eq!(r1, r2);
                    assert_eq!(u1, u2);
                }
                (LLMStreamChunk::Error { error: e1 }, LLMStreamChunk::Error { error: e2 }) => {
                    assert_eq!(e1, e2);
                }
                _ => {} // 其他变体需要类似的检查
            }
        }
    }

    #[test]
    fn test_provider_config_creation() {
        let config = LLMProviderConfig {
            provider_type: LLMProviderType::OpenAI,
            api_key: "test-key".to_string(),
            api_url: Some("https://api.openai.com/v1".to_string()),
            model: "gpt-4".to_string(),
            options: Some(std::collections::HashMap::new()),
        };

        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, "test-key");
        assert!(config.api_url.is_some());
    }

    #[test]
    fn test_tool_definition() {
        let tool = LLMTool {
            name: "test_function".to_string(),
            description: "A test function".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string",
                        "description": "First parameter"
                    }
                },
                "required": ["param1"]
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: LLMTool = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test_function");
        assert_eq!(deserialized.description, "A test function");
    }

    #[test]
    fn test_llm_request_validation() {
        // 测试基本请求结构
        let request = LLMRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("Hello".to_string()),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        // 基本验证 - 只检查结构是否正确
        assert!(!request.model.is_empty());
        assert!(!request.messages.is_empty());
        assert!(request.temperature.unwrap() >= 0.0 && request.temperature.unwrap() <= 2.0);
        assert!(request.max_tokens.unwrap() > 0);
    }

    #[test]
    fn test_multimodal_message() {
        let multimodal_message = LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Parts(vec![
                LLMMessagePart::Text {
                    text: "What's in this image?".to_string(),
                },
                LLMMessagePart::File {
                    mime_type: "image/jpeg".to_string(),
                    data: "base64data".to_string(),
                },
            ]),
        };

        let json = serde_json::to_string(&multimodal_message).unwrap();
        let deserialized: LLMMessage = serde_json::from_str(&json).unwrap();

        match deserialized.content {
            LLMMessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    LLMMessagePart::Text { text } => {
                        assert_eq!(text, "What's in this image?");
                    }
                    _ => panic!("Expected text part"),
                }
                match &parts[1] {
                    LLMMessagePart::File { mime_type, data } => {
                        assert_eq!(mime_type, "image/jpeg");
                        assert!(!data.is_empty());
                    }
                    _ => panic!("Expected file part"),
                }
            }
            _ => panic!("Expected parts content"),
        }
    }

    #[test]
    fn test_tool_call_message() {
        let tool_call_message = LLMMessage {
            role: "assistant".to_string(),
            content: LLMMessageContent::Parts(vec![
                LLMMessagePart::Text {
                    text: "I'll help you with that calculation.".to_string(),
                },
                LLMMessagePart::ToolCall {
                    tool_call_id: "call_123".to_string(),
                    tool_name: "calculator".to_string(),
                    args: serde_json::json!({
                        "operation": "add",
                        "a": 5,
                        "b": 3
                    }),
                },
            ]),
        };

        let json = serde_json::to_string(&tool_call_message).unwrap();
        let deserialized: LLMMessage = serde_json::from_str(&json).unwrap();

        match deserialized.content {
            LLMMessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[1] {
                    LLMMessagePart::ToolCall {
                        tool_call_id,
                        tool_name,
                        args,
                    } => {
                        assert_eq!(tool_call_id, "call_123");
                        assert_eq!(tool_name, "calculator");
                        assert_eq!(args["operation"], "add");
                    }
                    _ => panic!("Expected tool call part"),
                }
            }
            _ => panic!("Expected parts content"),
        }
    }
}
