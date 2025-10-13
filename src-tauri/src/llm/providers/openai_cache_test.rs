#[cfg(test)]
mod openai_cache_tests {
    use crate::llm::{
        providers::OpenAIProvider,
        types::{LLMMessage, LLMMessageContent, LLMMessagePart, LLMProviderConfig, LLMRequest},
    };
    use serde_json::Value;

    /// 测试 OpenAI 官方不应该注入 cache_control
    #[test]
    fn test_openai_official_no_cache_control() {
        let config = LLMProviderConfig {
            provider_type: "openai".to_string(),
            api_key: "test-key".to_string(),
            api_url: None,
            model: "gpt-4".to_string(),
            options: None,
            supports_prompt_cache: true, // 即使设置为 true
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("You are a helpful assistant.".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Hello".to_string()),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // 验证 system 消息没有 cache_control
        let system_msg = &messages[0];
        assert!(system_msg["content"].is_string()); // 应该是简单字符串，不是数组

        // 验证 user 消息没有 cache_control
        let user_msg = &messages[1];
        assert!(user_msg["content"].is_string()); // 应该是简单字符串，不是数组
    }

    /// 测试 OpenAI Compatible 应该注入 cache_control
    #[test]
    fn test_openai_compatible_with_cache_control() {
        let config = LLMProviderConfig {
            provider_type: "openai_compatible".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.litellm.ai/v1".to_string()),
            model: "anthropic/claude-3-5-sonnet".to_string(),
            options: None,
            supports_prompt_cache: true,
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "anthropic/claude-3-5-sonnet".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("You are a helpful assistant.".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("First message".to_string()),
                },
                LLMMessage {
                    role: "assistant".to_string(),
                    content: LLMMessageContent::Text("Response".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Second message".to_string()),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // 验证 system 消息有 cache_control
        let system_msg = &messages[0];
        assert!(system_msg["content"].is_array());
        let system_content = system_msg["content"].as_array().unwrap();
        assert_eq!(system_content[0]["type"], "text");
        assert!(system_content[0]["cache_control"].is_object());
        assert_eq!(system_content[0]["cache_control"]["type"], "ephemeral");

        // 验证第一条 user 消息有 cache_control（倒数第二条）
        let first_user_msg = &messages[1];
        assert!(first_user_msg["content"].is_array());
        let first_user_content = first_user_msg["content"].as_array().unwrap();
        assert!(first_user_content[0]["cache_control"].is_object());

        // 验证 assistant 消息没有 cache_control
        let assistant_msg = &messages[2];
        assert!(assistant_msg["content"].is_string());

        // 验证第二条 user 消息有 cache_control（最后一条）
        let second_user_msg = &messages[3];
        assert!(second_user_msg["content"].is_array());
        let second_user_content = second_user_msg["content"].as_array().unwrap();
        assert!(second_user_content[0]["cache_control"].is_object());
    }

    /// 测试 OpenAI Compatible 但未启用缓存
    #[test]
    fn test_openai_compatible_cache_disabled() {
        let config = LLMProviderConfig {
            provider_type: "openai_compatible".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.litellm.ai/v1".to_string()),
            model: "gpt-4".to_string(),
            options: None,
            supports_prompt_cache: false, // 未启用
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("You are a helpful assistant.".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Hello".to_string()),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // 验证所有消息都没有 cache_control
        for msg in messages {
            if msg["content"].is_string() {
                // 简单字符串格式，没有 cache_control
                continue;
            }
            if let Some(content_array) = msg["content"].as_array() {
                for part in content_array {
                    assert!(part["cache_control"].is_null());
                }
            }
        }
    }

    /// 测试多条 user 消息时的缓存标记策略
    #[test]
    fn test_cache_control_on_last_two_user_messages() {
        let config = LLMProviderConfig {
            provider_type: "openai_compatible".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.openrouter.ai/v1".to_string()),
            model: "anthropic/claude-3-5-sonnet".to_string(),
            options: None,
            supports_prompt_cache: true,
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "anthropic/claude-3-5-sonnet".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("System prompt".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("First user message".to_string()),
                },
                LLMMessage {
                    role: "assistant".to_string(),
                    content: LLMMessageContent::Text("First response".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Second user message".to_string()),
                },
                LLMMessage {
                    role: "assistant".to_string(),
                    content: LLMMessageContent::Text("Second response".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Third user message".to_string()),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // System 消息（索引 0）应该有 cache_control
        let system_msg = &messages[0];
        assert!(system_msg["content"].is_array());
        let system_content = system_msg["content"].as_array().unwrap();
        assert!(system_content[0]["cache_control"].is_object());

        // 第一条 user 消息（索引 1）应该没有 cache_control（不在最后两条中）
        let first_user = &messages[1];
        assert!(first_user["content"].is_string()); // 简单字符串，无缓存

        // 第二条 user 消息（索引 3，倒数第二条 user）应该有 cache_control
        let second_user = &messages[3];
        assert!(second_user["content"].is_array());
        let second_user_content = second_user["content"].as_array().unwrap();
        assert!(second_user_content[0]["cache_control"].is_object());

        // 第三条 user 消息（索引 5，最后一条 user）应该有 cache_control
        let third_user = &messages[5];
        assert!(third_user["content"].is_array());
        let third_user_content = third_user["content"].as_array().unwrap();
        assert!(third_user_content[0]["cache_control"].is_object());
    }

    /// 测试 Parts 格式消息的缓存控制
    #[test]
    fn test_cache_control_with_parts_content() {
        let config = LLMProviderConfig {
            provider_type: "openai_compatible".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.litellm.ai/v1".to_string()),
            model: "anthropic/claude-3-5-sonnet".to_string(),
            options: None,
            supports_prompt_cache: true,
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "anthropic/claude-3-5-sonnet".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("System".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Parts(vec![
                        LLMMessagePart::Text {
                            text: "First part".to_string(),
                            cache_control: None,
                        },
                        LLMMessagePart::Text {
                            text: "Second part".to_string(),
                            cache_control: None,
                        },
                    ]),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // User 消息应该有 Parts，且最后一个 part 有 cache_control
        let user_msg = &messages[1];
        assert!(user_msg["content"].is_array());
        let parts = user_msg["content"].as_array().unwrap();
        assert_eq!(parts.len(), 2);

        // 第一个 part 不应该有 cache_control
        assert!(parts[0]["cache_control"].is_null());

        // 最后一个 part 应该有 cache_control
        assert!(parts[1]["cache_control"].is_object());
        assert_eq!(parts[1]["cache_control"]["type"], "ephemeral");
    }

    /// 测试组合场景：openai + supports_prompt_cache=true 仍然不注入
    #[test]
    fn test_openai_official_ignores_cache_flag() {
        let config = LLMProviderConfig {
            provider_type: "openai".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.openai.com/v1".to_string()),
            model: "gpt-4o".to_string(),
            options: None,
            supports_prompt_cache: true, // 即使为 true
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("System".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Hello".to_string()),
                },
            ],
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // 所有消息都应该是简单字符串格式，没有 cache_control
        for msg in messages {
            assert!(
                msg["content"].is_string(),
                "OpenAI 官方不应该使用结构化 content 和 cache_control"
            );
        }
    }

    /// 测试边界情况：只有一条 user 消息
    #[test]
    fn test_single_user_message_with_cache() {
        let config = LLMProviderConfig {
            provider_type: "openai_compatible".to_string(),
            api_key: "test-key".to_string(),
            api_url: Some("https://api.litellm.ai/v1".to_string()),
            model: "anthropic/claude-3-5-sonnet".to_string(),
            options: None,
            supports_prompt_cache: true,
        };

        let provider = OpenAIProvider::new(config);
        let request = LLMRequest {
            model: "anthropic/claude-3-5-sonnet".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: LLMMessageContent::Text("System".to_string()),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: LLMMessageContent::Text("Only user message".to_string()),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            tool_choice: None,
            stream: false,
        };

        let body = provider.build_body(&request);
        let messages = body["messages"].as_array().unwrap();

        // System 消息应该有 cache_control
        let system_msg = &messages[0];
        assert!(system_msg["content"].is_array());
        let system_content = system_msg["content"].as_array().unwrap();
        assert!(system_content[0]["cache_control"].is_object());

        // 唯一的 user 消息应该有 cache_control（作为最后一条）
        let user_msg = &messages[1];
        assert!(user_msg["content"].is_array());
        let user_content = user_msg["content"].as_array().unwrap();
        assert!(user_content[0]["cache_control"].is_object());
    }
}
