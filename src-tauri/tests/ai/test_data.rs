/*!
 * AI模块测试数据
 *
 * 提供各种测试场景所需的预定义数据
 */

use std::collections::HashMap;
use termx::ai::{
    AIContext, AIModelConfig, AIModelOptions, AIProvider, AIRequest, AIRequestType, AIResponse,
    AISettings, CacheConfig, CommandAlternative, CommandExplanation, CommandPart, ErrorAnalysis,
    RiskLevel, RiskWarning, SystemInfo,
};

/// 测试用的AI模型配置数据
pub struct TestModelConfigs;

impl TestModelConfigs {
    /// OpenAI模型配置
    pub fn openai() -> AIModelConfig {
        AIModelConfig {
            id: "openai-gpt4".to_string(),
            name: "OpenAI GPT-4".to_string(),
            provider: AIProvider::OpenAI,
            api_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-key-openai".to_string(),
            model: "gpt-4".to_string(),
            is_default: Some(true),
            options: Some(AIModelOptions {
                temperature: Some(0.7),
                max_tokens: Some(4096),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                custom_parameters: HashMap::new(),
            }),
        }
    }

    /// Claude模型配置
    pub fn claude() -> AIModelConfig {
        AIModelConfig {
            id: "claude-3".to_string(),
            name: "Claude 3 Sonnet".to_string(),
            provider: AIProvider::Claude,
            api_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-test-key-claude".to_string(),
            model: "claude-3-sonnet-20240229".to_string(),
            is_default: Some(false),
            options: Some(AIModelOptions {
                temperature: Some(0.5),
                max_tokens: Some(8192),
                top_p: Some(0.9),
                frequency_penalty: None,
                presence_penalty: None,
                custom_parameters: HashMap::new(),
            }),
        }
    }

    /// 本地模型配置
    pub fn local() -> AIModelConfig {
        AIModelConfig {
            id: "local-llama".to_string(),
            name: "Local Llama 2".to_string(),
            provider: AIProvider::Local,
            api_url: "http://localhost:11434".to_string(),
            api_key: "".to_string(),
            model: "llama2".to_string(),

            is_default: Some(false),
            options: Some(AIModelOptions {
                temperature: Some(0.8),
                max_tokens: Some(2048),
                top_p: Some(0.95),
                frequency_penalty: None,
                presence_penalty: None,
                custom_parameters: {
                    let mut params = HashMap::new();
                    params.insert("num_ctx".to_string(), "4096".to_string());
                    params.insert("repeat_penalty".to_string(), "1.1".to_string());
                    params
                },
            }),
        }
    }

    /// 自定义模型配置
    pub fn custom() -> AIModelConfig {
        AIModelConfig {
            id: "custom-api".to_string(),
            name: "Custom API Model".to_string(),
            provider: AIProvider::Custom,
            api_url: "https://custom-api.example.com/v1".to_string(),
            api_key: "custom-api-key".to_string(),
            model: "custom-model-v1".to_string(),

            is_default: Some(false),
            options: None,
        }
    }

    /// 获取所有测试模型配置
    pub fn all() -> Vec<AIModelConfig> {
        vec![
            Self::openai(),
            Self::claude(),
            Self::local(),
            Self::custom(),
        ]
    }
}

/// 测试用的AI请求数据
pub struct TestRequests;

impl TestRequests {
    /// 聊天请求
    pub fn chat() -> AIRequest {
        AIRequest::new(AIRequestType::Chat, "How do I fix this error?".to_string())
            .with_context(TestContexts::error_context())
    }

    /// 命令解释请求
    pub fn explanation() -> AIRequest {
        AIRequest::new(AIRequestType::Explanation, "rm -rf /".to_string())
            .with_context(TestContexts::basic())
    }

    /// 错误分析请求
    pub fn error_analysis() -> AIRequest {
        AIRequest::new(
            AIRequestType::ErrorAnalysis,
            "TypeError: Cannot read property 'length' of undefined".to_string(),
        )
        .with_context(TestContexts::javascript_context())
    }

    /// 空内容请求（用于测试验证）
    pub fn empty() -> AIRequest {
        AIRequest::new(AIRequestType::Chat, "".to_string())
    }

    /// 长内容请求（用于测试性能）
    pub fn long_content() -> AIRequest {
        let content = "a".repeat(10000);
        AIRequest::new(AIRequestType::Chat, content)
    }
}

/// 测试用的AI上下文数据
pub struct TestContexts;

impl TestContexts {
    /// 基本上下文
    pub fn basic() -> AIContext {
        AIContext {
            working_directory: Some("/home/user".to_string()),
            command_history: Some(vec!["ls".to_string(), "pwd".to_string()]),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("USER".to_string(), "testuser".to_string());
                env.insert("HOME".to_string(), "/home/user".to_string());
                env
            }),
            current_command: None,
            last_output: None,
            system_info: Some(SystemInfo {
                platform: "linux".to_string(),
                shell: "bash".to_string(),
                user: "testuser".to_string(),
            }),
        }
    }

    /// 开发环境上下文
    pub fn development() -> AIContext {
        AIContext {
            working_directory: Some("/home/user/project".to_string()),
            command_history: Some(vec![
                "git clone https://github.com/user/project.git".to_string(),
                "cd project".to_string(),
                "npm install".to_string(),
                "npm run dev".to_string(),
            ]),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("NODE_ENV".to_string(), "development".to_string());
                env.insert(
                    "PATH".to_string(),
                    "/usr/local/bin:/usr/bin:/bin".to_string(),
                );
                env
            }),
            current_command: Some("npm install".to_string()),
            last_output: Some("Installing dependencies...".to_string()),
            system_info: Some(SystemInfo {
                platform: "darwin".to_string(),
                shell: "zsh".to_string(),
                user: "developer".to_string(),
            }),
        }
    }

    /// 错误上下文
    pub fn error_context() -> AIContext {
        AIContext {
            working_directory: Some("/home/user/project".to_string()),
            command_history: Some(vec!["npm test".to_string(), "npm run build".to_string()]),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("NODE_ENV".to_string(), "test".to_string());
                env
            }),
            current_command: Some("npm test".to_string()),
            last_output: Some("Error: Test failed with exit code 1".to_string()),
            system_info: Some(SystemInfo {
                platform: "linux".to_string(),
                shell: "bash".to_string(),
                user: "ci".to_string(),
            }),
        }
    }

    /// JavaScript项目上下文
    pub fn javascript_context() -> AIContext {
        AIContext {
            working_directory: Some("/home/user/js-project".to_string()),
            command_history: Some(vec!["node app.js".to_string(), "npm run test".to_string()]),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("NODE_VERSION".to_string(), "18.17.0".to_string());
                env.insert("NPM_VERSION".to_string(), "9.6.7".to_string());
                env
            }),
            current_command: Some("node app.js".to_string()),
            last_output: Some(
                "TypeError: Cannot read property 'length' of undefined\n    at app.js:15:20"
                    .to_string(),
            ),
            system_info: Some(SystemInfo {
                platform: "linux".to_string(),
                shell: "bash".to_string(),
                user: "developer".to_string(),
            }),
        }
    }
}

/// 测试用的AI响应数据
pub struct TestResponses;

impl TestResponses {
    /// 命令解释响应
    pub fn explanation() -> CommandExplanation {
        CommandExplanation {
            command: "rm -rf /".to_string(),
            explanation: "This command recursively removes all files and directories starting from the root directory.".to_string(),
            breakdown: Some(vec![
                CommandPart {
                    part: "rm".to_string(),
                    description: "Remove files and directories".to_string(),
                },
                CommandPart {
                    part: "-r".to_string(),
                    description: "Recursive - remove directories and their contents".to_string(),
                },
                CommandPart {
                    part: "-f".to_string(),
                    description: "Force - ignore nonexistent files and never prompt".to_string(),
                },
                CommandPart {
                    part: "/".to_string(),
                    description: "Root directory - the top level of the filesystem".to_string(),
                },
            ]),
            risks: Some(vec![
                RiskWarning {
                    level: RiskLevel::Critical,
                    description: "This command will delete ALL files on the system and render it unusable.".to_string(),
                },
            ]),
            alternatives: Some(vec![
                CommandAlternative {
                    command: "rm -rf ./target_directory".to_string(),
                    description: "Remove a specific directory instead of everything".to_string(),
                },
            ]),
        }
    }

    /// 错误分析响应
    pub fn error_analysis() -> ErrorAnalysis {
        ErrorAnalysis {
            error_type: "TypeError".to_string(),
            description: "Attempting to access a property of an undefined value".to_string(),
            possible_causes: vec![
                "Variable was not initialized".to_string(),
                "Function returned undefined".to_string(),
                "Object property does not exist".to_string(),
            ],
            solutions: vec![
                "Check if the variable is defined before accessing its properties".to_string(),
                "Use optional chaining (?.) to safely access properties".to_string(),
                "Initialize the variable with a default value".to_string(),
            ],
            code_examples: Some(vec![
                "if (myVar && myVar.length) { /* safe to use */ }".to_string(),
                "const length = myVar?.length || 0;".to_string(),
            ]),
            related_docs: Some(vec![
                "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Optional_chaining".to_string(),
            ]),
        }
    }

    /// 简单聊天响应
    pub fn chat() -> AIResponse {
        AIResponse {
            content: "I can help you fix that error. Can you provide more details about the specific error message and the code that's causing it?".to_string(),
            model_id: "test-model".to_string(),
            usage: None,
            metadata: None,
        }
    }
}

/// 测试用的缓存配置
pub struct TestCacheConfigs;

impl TestCacheConfigs {
    /// 默认缓存配置
    pub fn default() -> CacheConfig {
        CacheConfig::default()
    }

    /// 小容量缓存配置（用于测试LRU）
    pub fn small_capacity() -> CacheConfig {
        CacheConfig {
            memory_capacity: 3,
            default_ttl: 300,
            type_ttl: HashMap::new(),
            enable_disk_cache: false,
            disk_cache_path: None,
            max_disk_size: 0,
        }
    }

    /// 短TTL缓存配置（用于测试过期）
    pub fn short_ttl() -> CacheConfig {
        CacheConfig {
            memory_capacity: 100,
            default_ttl: 1, // 1秒
            type_ttl: {
                let mut ttl_map = HashMap::new();

                ttl_map.insert(AIRequestType::Chat, 1);
                ttl_map
            },
            enable_disk_cache: false,
            disk_cache_path: None,
            max_disk_size: 0,
        }
    }

    /// 磁盘缓存配置
    pub fn with_disk_cache(path: String) -> CacheConfig {
        CacheConfig {
            memory_capacity: 50,
            default_ttl: 3600,
            type_ttl: HashMap::new(),
            enable_disk_cache: true,
            disk_cache_path: Some(path),
            max_disk_size: 100, // 100MB
        }
    }
}

/// 测试用的AI设置
pub struct TestSettings;

impl TestSettings {
    /// 完整的测试设置
    pub fn complete() -> AISettings {
        let mut settings = AISettings::default();
        settings.models = TestModelConfigs::all();
        settings.default_model_id = Some("openai-gpt4".to_string());
        settings
    }

    /// 空设置
    pub fn empty() -> AISettings {
        AISettings::default()
    }

    /// 只有OpenAI模型的设置
    pub fn openai_only() -> AISettings {
        let mut settings = AISettings::default();
        settings.models = vec![TestModelConfigs::openai()];
        settings.default_model_id = Some("openai-gpt4".to_string());
        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_configs() {
        let configs = TestModelConfigs::all();
        assert_eq!(configs.len(), 4);

        let openai = TestModelConfigs::openai();
        assert_eq!(openai.provider, AIProvider::OpenAI);
        assert_eq!(openai.is_default, Some(true));
    }

    #[test]
    fn test_requests() {
        let chat = TestRequests::chat();
        assert_eq!(chat.request_type, AIRequestType::Chat);
        assert!(chat.context.is_some());

        let empty = TestRequests::empty();
        assert!(empty.content.is_empty());
    }

    #[test]
    fn test_contexts() {
        let basic = TestContexts::basic();
        assert!(basic.working_directory.is_some());
        assert!(basic.system_info.is_some());

        let dev = TestContexts::development();
        assert!(dev.command_history.is_some());
        assert!(dev.environment.is_some());
    }

    #[test]
    fn test_responses() {
        let explanation = TestResponses::explanation();
        assert!(explanation.risks.is_some());
        assert!(explanation.alternatives.is_some());
    }

    #[test]
    fn test_cache_configs() {
        let small = TestCacheConfigs::small_capacity();
        assert_eq!(small.memory_capacity, 3);

        let short = TestCacheConfigs::short_ttl();
        assert_eq!(short.default_ttl, 1);
    }

    #[test]
    fn test_settings() {
        let complete = TestSettings::complete();
        assert_eq!(complete.models.len(), 4);
        assert!(complete.default_model_id.is_some());

        let empty = TestSettings::empty();
        assert!(empty.models.is_empty());
        assert!(empty.default_model_id.is_none());
    }
}
