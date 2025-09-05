#[cfg(test)]
mod tests {
    use crate::llm::types::*;

    #[test]
    fn test_message_content_serialization() {
        // Test text content
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

        // Test parts content
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
            LLMStreamChunk::TextStart {
                stream_id: "test-123".to_string(),
            },
            LLMStreamChunk::TextDelta {
                stream_id: "test-123".to_string(),
                delta: "Hello".to_string(),
                text: "Hello".to_string(),
            },
            LLMStreamChunk::TextEnd {
                stream_id: "test-123".to_string(),
                text: "Hello world".to_string(),
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

            // Basic check that serialization/deserialization works
            match (&chunk, &deserialized) {
                (
                    LLMStreamChunk::TextStart { stream_id: id1 },
                    LLMStreamChunk::TextStart { stream_id: id2 },
                ) => {
                    assert_eq!(id1, id2);
                }
                (LLMStreamChunk::Error { error: e1 }, LLMStreamChunk::Error { error: e2 }) => {
                    assert_eq!(e1, e2);
                }
                _ => {} // Other variants would need similar checks
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
        // Test basic request structure
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

        // Basic validation - just check the structure is correct
        assert!(!request.model.is_empty());
        assert!(!request.messages.is_empty());
        assert!(request.temperature.unwrap() >= 0.0 && request.temperature.unwrap() <= 2.0);
        assert!(request.max_tokens.unwrap() > 0);
    }

    #[test]
    fn test_error_types() {
        let errors = vec![
            LLMError::Provider("Test provider error".to_string()),
            LLMError::Config("Test config error".to_string()),
            LLMError::Network("Test network error".to_string()),
            LLMError::ModelNotFound("test-model".to_string()),
            LLMError::UnsupportedProvider("test-provider".to_string()),
            LLMError::InvalidResponse("Invalid JSON".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
        }
    }
}
