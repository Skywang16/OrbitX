#[cfg(test)]
mod integration_tests {
    use super::super::{provider::*, types::*};
    use std::env;
    use tokio_stream::StreamExt;

    /// 集成测试需要真实的 API 密钥
    /// 设置环境变量 OPENAI_API_KEY 来运行这些测试
    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn test_openai_integration() {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::OpenAI,
            api_key,
            api_url: None,
            model: "gpt-3.5-turbo".to_string(),
            options: None,
        };

        let provider = OpenAIProvider::new(&config).expect("Failed to create provider");

        let request = LLMRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("Say hello in one word".to_string()),
            }],
            temperature: Some(0.1),
            max_tokens: Some(10),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let response = provider.call(request).await.expect("API call failed");

        assert!(!response.content.is_empty());
        println!("OpenAI Response: {}", response.content);
    }

    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn test_openai_streaming() {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::OpenAI,
            api_key,
            api_url: None,
            model: "gpt-3.5-turbo".to_string(),
            options: None,
        };

        let provider = OpenAIProvider::new(&config).expect("Failed to create provider");

        let request = LLMRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("Count from 1 to 3".to_string()),
            }],
            temperature: Some(0.1),
            max_tokens: Some(50),
            tools: None,
            tool_choice: None,
            stream: true,
        };

        let mut stream = provider
            .call_stream(request)
            .await
            .expect("Stream call failed");
        let mut chunks = Vec::new();
        let mut chunk_count = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.expect("Chunk error");
            chunks.push(chunk);
            chunk_count += 1;

            // Stop after getting a few chunks to avoid long test
            if chunk_count > 10 {
                break;
            }
        }

        assert!(!chunks.is_empty());
        println!("Received {} chunks from OpenAI", chunks.len());
    }

    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn test_anthropic_integration() {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::Anthropic,
            api_key,
            api_url: None,
            model: "claude-3-haiku-20240307".to_string(),
            options: None,
        };

        let provider = AnthropicProvider::new(&config).expect("Failed to create provider");

        let request = LLMRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("You are a helpful assistant.".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Say hello in one word".to_string()),
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(10),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let response = provider.call(request).await.expect("API call failed");

        assert!(!response.content.is_empty());
        println!("Anthropic Response: {}", response.content);
    }

    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn test_anthropic_streaming() {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::Anthropic,
            api_key,
            api_url: None,
            model: "claude-3-haiku-20240307".to_string(),
            options: None,
        };

        let provider = AnthropicProvider::new(&config).expect("Failed to create provider");

        let request = LLMRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("You are a helpful assistant.".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Count from 1 to 3".to_string()),
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(50),
            tools: None,
            tool_choice: None,
            stream: true,
        };

        let mut stream = provider
            .call_stream(request)
            .await
            .expect("Stream call failed");
        let mut chunks = Vec::new();
        let mut chunk_count = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.expect("Chunk error");
            chunks.push(chunk);
            chunk_count += 1;

            // Stop after getting a few chunks to avoid long test
            if chunk_count > 10 {
                break;
            }
        }

        assert!(!chunks.is_empty());
        println!("Received {} chunks from Anthropic", chunks.len());
    }

    #[tokio::test]
    async fn test_tool_calling_format() {
        // Test tool calling message format
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

    #[tokio::test]
    async fn test_multimodal_message_format() {
        // Test multimodal message with text and image
        let multimodal_message = LLMMessage {
            role: "user".to_string(),
            content: LLMMessageContent::Parts(vec![
                LLMMessagePart::Text {
                    text: "What do you see in this image?".to_string(),
                },
                LLMMessagePart::File {
                    mime_type: "image/jpeg".to_string(),
                    data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
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
                        assert_eq!(text, "What do you see in this image?");
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

    #[tokio::test]
    #[ignore] // 需要真实 API 密钥
    async fn test_openai_with_tools() {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::OpenAI,
            api_key,
            api_url: None,
            model: "gpt-3.5-turbo".to_string(),
            options: None,
        };

        let provider = OpenAIProvider::new(&config).expect("Failed to create provider");

        let request = LLMRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("What's 15 + 27?".to_string()),
            }],
            temperature: Some(0.1),
            max_tokens: Some(100),
            tools: Some(vec![LLMTool {
                name: "calculator".to_string(),
                description: "Perform basic arithmetic operations".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"]
                        },
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["operation", "a", "b"]
                }),
            }]),
            tool_choice: Some("auto".to_string()),
            stream: false,
        };

        let response = provider.call(request).await.expect("API call failed");

        println!("Response with tools: {}", response.content);
        if let Some(tool_calls) = response.tool_calls {
            println!("Tool calls: {:?}", tool_calls);
        }
    }
}
