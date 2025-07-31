/*!
 * AI类型系统测试
 *
 * 测试AI模块中所有数据类型的序列化、反序列化、验证和转换功能
 */

use serde_json;
use std::collections::HashMap;
use termx::ai::{
    AIContext, AIModelConfig, AIModelOptions,
    AIPerformanceSettings, AIProvider, AIRequest, AIRequestType, AIResponse, AISettings,
    CommandAlternative, CommandExplanation, CommandPart, ErrorAnalysis, RiskLevel, RiskWarning,
    SystemInfo,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::*;

    /// 测试AI提供商枚举
    #[test]
    fn test_ai_provider_enum() {
        // 测试序列化
        let providers = vec![
            AIProvider::OpenAI,
            AIProvider::Claude,
            AIProvider::Local,
            AIProvider::Custom,
        ];

        for provider in providers {
            let serialized =
                serde_json::to_string(&provider).expect("Failed to serialize provider");
            let deserialized: AIProvider =
                serde_json::from_str(&serialized).expect("Failed to deserialize provider");
            assert_eq!(provider, deserialized);
        }

        // 测试字符串表示
        assert_eq!(
            serde_json::to_string(&AIProvider::OpenAI).unwrap(),
            "\"openai\""
        );
        assert_eq!(
            serde_json::to_string(&AIProvider::Claude).unwrap(),
            "\"claude\""
        );
        assert_eq!(
            serde_json::to_string(&AIProvider::Local).unwrap(),
            "\"local\""
        );
        assert_eq!(
            serde_json::to_string(&AIProvider::Custom).unwrap(),
            "\"custom\""
        );
    }

    /// 测试AI请求类型枚举
    #[test]
    fn test_ai_request_type_enum() {
        let request_types = vec![
            AIRequestType::Chat,
            AIRequestType::Explanation,
            AIRequestType::ErrorAnalysis,
        ];

        for request_type in request_types {
            let serialized =
                serde_json::to_string(&request_type).expect("Failed to serialize request type");
            let deserialized: AIRequestType =
                serde_json::from_str(&serialized).expect("Failed to deserialize request type");
            assert_eq!(request_type, deserialized);
        }
    }

    /// 测试AI模型配置
    #[test]
    fn test_ai_model_config() {
        let config = TestModelConfigs::openai();

        // 测试基本属性
        assert_eq!(config.id, "openai-gpt4");
        assert_eq!(config.provider, AIProvider::OpenAI);
        assert_eq!(config.is_default, Some(true));

        // 测试序列化为camelCase
        let serialized = serde_json::to_string(&config).expect("Failed to serialize model config");

        // 验证序列化后的JSON使用camelCase
        let json_value: serde_json::Value =
            serde_json::from_str(&serialized).expect("Failed to parse serialized JSON");

        assert!(json_value.get("apiUrl").is_some());
        assert!(json_value.get("apiKey").is_some());
        assert!(json_value.get("isDefault").is_some());

        // 测试反序列化
        let deserialized: AIModelConfig =
            serde_json::from_str(&serialized).expect("Failed to deserialize model config");

        assert_eq!(config.id, deserialized.id);
        assert_eq!(config.provider, deserialized.provider);
        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.is_default, deserialized.is_default);
    }

    /// 测试AI模型选项
    #[test]
    fn test_ai_model_options() {
        let mut custom_params = HashMap::new();
        custom_params.insert("custom_param".to_string(), "custom_value".to_string());

        let options = AIModelOptions {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            custom_parameters: custom_params,
        };

        // 测试序列化
        let serialized =
            serde_json::to_string(&options).expect("Failed to serialize model options");
        let deserialized: AIModelOptions =
            serde_json::from_str(&serialized).expect("Failed to deserialize model options");

        assert_eq!(options.temperature, deserialized.temperature);
        assert_eq!(options.max_tokens, deserialized.max_tokens);
        assert_eq!(options.custom_parameters, deserialized.custom_parameters);
    }

    /// 测试AI请求
    #[test]
    fn test_ai_request() {
        let request = TestRequests::chat();

        // 测试基本属性
        assert_eq!(request.request_type, AIRequestType::Chat);
        assert_eq!(request.content, "How do I fix this error?");
        assert!(request.context.is_some());

        // 测试验证
        assert!(request.validate().is_ok());

        // 测试空内容验证
        let empty_request = TestRequests::empty();
        assert!(empty_request.validate().is_err());

        // 测试序列化
        let serialized = serde_json::to_string(&request).expect("Failed to serialize request");
        let deserialized: AIRequest =
            serde_json::from_str(&serialized).expect("Failed to deserialize request");

        assert_eq!(request.request_type, deserialized.request_type);
        assert_eq!(request.content, deserialized.content);
    }

    /// 测试AI上下文
    #[test]
    fn test_ai_context() {
        let context = TestContexts::development();

        // 测试基本属性
        assert!(context.working_directory.is_some());
        assert!(context.command_history.is_some());
        assert!(context.environment.is_some());
        assert!(context.system_info.is_some());

        // 测试序列化
        let serialized = serde_json::to_string(&context).expect("Failed to serialize context");
        let deserialized: AIContext =
            serde_json::from_str(&serialized).expect("Failed to deserialize context");

        assert_eq!(context.working_directory, deserialized.working_directory);
        assert_eq!(context.command_history, deserialized.command_history);
        assert_eq!(context.environment, deserialized.environment);
    }

    /// 测试系统信息
    #[test]
    fn test_system_info() {
        let system_info = SystemInfo {
            platform: "linux".to_string(),
            shell: "bash".to_string(),
            user: "testuser".to_string(),
        };

        // 测试序列化
        let serialized =
            serde_json::to_string(&system_info).expect("Failed to serialize system info");
        let deserialized: SystemInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize system info");

        assert_eq!(system_info.platform, deserialized.platform);
        assert_eq!(system_info.shell, deserialized.shell);
        assert_eq!(system_info.user, deserialized.user);
    }

    /// 测试AI响应
    #[test]
    fn test_ai_response() {
        let response = TestResponses::chat();

        // 测试基本属性
        assert!(!response.content.is_empty());
        assert!(!response.model_id.is_empty());

        // 测试序列化
        let serialized = serde_json::to_string(&response).expect("Failed to serialize response");
        let deserialized: AIResponse =
            serde_json::from_str(&serialized).expect("Failed to deserialize response");

        assert_eq!(response.content, deserialized.content);
        assert_eq!(response.model_id, deserialized.model_id);
    }

    /// 测试命令解释
    #[test]
    fn test_command_explanation() {
        let explanation = TestResponses::explanation();

        // 测试基本属性
        assert_eq!(explanation.command, "rm -rf /");
        assert!(explanation.breakdown.is_some());
        assert!(explanation.risks.is_some());
        assert!(explanation.alternatives.is_some());

        // 测试风险警告
        let risks = explanation.risks.unwrap();
        assert!(!risks.is_empty());
        assert_eq!(risks[0].level, RiskLevel::Critical);

        // 测试序列化
        let serialized =
            serde_json::to_string(&explanation).expect("Failed to serialize explanation");
        let deserialized: CommandExplanation =
            serde_json::from_str(&serialized).expect("Failed to deserialize explanation");

        assert_eq!(explanation.command, deserialized.command);
        assert_eq!(explanation.explanation, deserialized.explanation);
    }

    /// 测试风险级别枚举
    #[test]
    fn test_risk_level_enum() {
        let levels = vec![
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Critical,
        ];

        for level in levels {
            let serialized = serde_json::to_string(&level).expect("Failed to serialize risk level");
            let deserialized: RiskLevel =
                serde_json::from_str(&serialized).expect("Failed to deserialize risk level");
            assert_eq!(level, deserialized);
        }
    }

    /// 测试错误分析
    #[test]
    fn test_error_analysis() {
        let analysis = TestResponses::error_analysis();

        // 测试基本属性
        assert_eq!(analysis.error_type, "TypeError");
        assert!(!analysis.possible_causes.is_empty());
        assert!(!analysis.solutions.is_empty());
        assert!(analysis.code_examples.is_some());
        assert!(analysis.related_docs.is_some());

        // 测试序列化
        let serialized =
            serde_json::to_string(&analysis).expect("Failed to serialize error analysis");
        let deserialized: ErrorAnalysis =
            serde_json::from_str(&serialized).expect("Failed to deserialize error analysis");

        assert_eq!(analysis.error_type, deserialized.error_type);
        assert_eq!(analysis.possible_causes, deserialized.possible_causes);
        assert_eq!(analysis.solutions, deserialized.solutions);
    }



        assert_eq!(completion.items.len(), deserialized.items.len());
        assert_eq!(completion.replace_start, deserialized.replace_start);
        assert_eq!(completion.replace_end, deserialized.replace_end);
    }

    /// 测试AI设置
    #[test]
    fn test_ai_settings() {
        let settings = TestSettings::complete();

        // 测试基本属性
        assert_eq!(settings.models.len(), 4);
        assert_eq!(settings.default_model_id, Some("openai-gpt4".to_string()));

        // 测试功能设置
        assert!(settings.features.completion.enabled);
        assert!(settings.features.chat.enabled);
        assert!(settings.features.explanation.enabled);
        assert!(settings.features.error_analysis.enabled);

        // 测试序列化
        let serialized = serde_json::to_string(&settings).expect("Failed to serialize settings");
        let deserialized: AISettings =
            serde_json::from_str(&serialized).expect("Failed to deserialize settings");

        assert_eq!(settings.models.len(), deserialized.models.len());
        assert_eq!(settings.default_model_id, deserialized.default_model_id);
    }

    /// 测试默认值实现
    #[test]
    fn test_default_implementations() {
        // 测试AIContext默认值
        let default_context = AIContext::default();
        assert!(default_context.working_directory.is_none());
        assert!(default_context.command_history.is_none());
        assert!(default_context.environment.is_none());

        // 测试AISettings默认值
        let default_settings = AISettings::default();
        assert!(default_settings.models.is_empty());
        assert!(default_settings.default_model_id.is_none());
        assert!(default_settings.features.completion.enabled);
    }

    /// 测试类型的Clone实现
    #[test]
    fn test_clone_implementations() {
        let config = TestModelConfigs::openai();
        let cloned_config = config.clone();
        assert_eq!(config.id, cloned_config.id);
        assert_eq!(config.provider, cloned_config.provider);

        let request = TestRequests::completion();
        let cloned_request = request.clone();
        assert_eq!(request.content, cloned_request.content);
        assert_eq!(request.request_type, cloned_request.request_type);

        let response = TestResponses::chat();
        let cloned_response = response.clone();
        assert_eq!(response.content, cloned_response.content);
        assert_eq!(response.model_id, cloned_response.model_id);
    }

    /// 测试类型的Debug实现
    #[test]
    fn test_debug_implementations() {
        let config = TestModelConfigs::openai();
        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("AIModelConfig"));
        assert!(debug_string.contains(&config.id));

        let request = TestRequests::completion();
        let debug_string = format!("{:?}", request);
        assert!(debug_string.contains("AIRequest"));

        let provider = AIProvider::OpenAI;
        let debug_string = format!("{:?}", provider);
        assert!(debug_string.contains("OpenAI"));
    }

    /// 测试复杂嵌套结构的序列化
    #[test]
    fn test_complex_nested_serialization() {
        let settings = TestSettings::complete();

        // 序列化整个设置对象
        let serialized =
            serde_json::to_string_pretty(&settings).expect("Failed to serialize complex settings");

        // 反序列化
        let deserialized: AISettings =
            serde_json::from_str(&serialized).expect("Failed to deserialize complex settings");

        // 验证嵌套结构
        assert_eq!(settings.models.len(), deserialized.models.len());
        for (original, deserialized) in settings.models.iter().zip(deserialized.models.iter()) {
            assert_eq!(original.id, deserialized.id);
            assert_eq!(original.provider, deserialized.provider);
            assert_eq!(original.options.is_some(), deserialized.options.is_some());
        }

        // 验证功能设置
        assert_eq!(
            settings.features.completion.enabled,
            deserialized.features.completion.enabled
        );
        assert_eq!(
            settings.features.chat.max_history_length,
            deserialized.features.chat.max_history_length
        );
    }

    /// 测试camelCase序列化格式
    #[test]
    fn test_camel_case_serialization() {
        // 测试AIRequest序列化
        let request = AIRequest {
            request_type: AIRequestType::Chat,
            content: "test".to_string(),
            context: Some(AIContext {
                working_directory: Some("/test".to_string()),
                command_history: Some(vec!["ls".to_string()]),
                environment: Some(HashMap::new()),
                current_command: Some("pwd".to_string()),
                last_output: Some("output".to_string()),
                system_info: Some(SystemInfo {
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    shell: "bash".to_string(),
                }),
            }),
            options: Some(AIRequestOptions {
                max_tokens: Some(100),
                temperature: Some(0.7),
                stream: Some(false),
            }),
        };

        let serialized = serde_json::to_string(&request).expect("Failed to serialize request");
        let json_value: serde_json::Value =
            serde_json::from_str(&serialized).expect("Failed to parse serialized JSON");

        // 验证主要字段使用camelCase
        assert!(json_value.get("requestType").is_some());
        assert!(json_value.get("content").is_some());
        assert!(json_value.get("context").is_some());
        assert!(json_value.get("options").is_some());

        // 验证嵌套对象也使用camelCase
        let context = json_value.get("context").unwrap();
        assert!(context.get("workingDirectory").is_some());
        assert!(context.get("commandHistory").is_some());
        assert!(context.get("currentCommand").is_some());
        assert!(context.get("lastOutput").is_some());
        assert!(context.get("systemInfo").is_some());

        let options = json_value.get("options").unwrap();
        assert!(options.get("maxTokens").is_some());
        assert!(options.get("temperature").is_some());
        assert!(options.get("stream").is_some());

        // 测试反序列化
        let deserialized: AIRequest =
            serde_json::from_str(&serialized).expect("Failed to deserialize request");
        assert_eq!(request.content, deserialized.content);
    }

    /// 测试AIResponse序列化
    #[test]
    fn test_ai_response_serialization() {
        let response = AIResponse {
            content: "test response".to_string(),
            response_type: AIResponseType::Text,
            suggestions: Some(vec!["suggestion1".to_string()]),
            metadata: Some(AIResponseMetadata {
                model: Some("gpt-4".to_string()),
                tokens_used: Some(50),
                response_time: Some(1000),
            }),
        };

        let serialized = serde_json::to_string(&response).expect("Failed to serialize response");
        let json_value: serde_json::Value =
            serde_json::from_str(&serialized).expect("Failed to parse serialized JSON");

        // 验证字段使用camelCase
        assert!(json_value.get("content").is_some());
        assert!(json_value.get("responseType").is_some());
        assert!(json_value.get("suggestions").is_some());
        assert!(json_value.get("metadata").is_some());

        // 验证metadata字段
        let metadata = json_value.get("metadata").unwrap();
        assert!(metadata.get("model").is_some());
        assert!(metadata.get("tokensUsed").is_some());
        assert!(metadata.get("responseTime").is_some());
    }
}
