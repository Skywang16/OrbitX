use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm::types::LLMProviderType;

/// 模型特殊能力标志
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 是否支持视觉输入
    pub supports_vision: bool,
    /// 是否支持流式输出
    pub supports_streaming: bool,
    /// 是否为推理模型（如o1系列）
    pub is_reasoning_model: bool,
    /// 最大上下文长度
    pub max_context_tokens: u32,
    /// 推荐的温度范围
    pub temperature_range: Option<(f32, f32)>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            is_reasoning_model: false,
            max_context_tokens: 4096,
            temperature_range: Some((0.0, 2.0)),
        }
    }
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// 模型ID
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// 模型能力
    pub capabilities: ModelCapabilities,
    /// 是否已弃用
    pub deprecated: bool,
}

/// 供应商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    /// 供应商类型
    pub provider_type: LLMProviderType,
    /// 显示名称
    pub display_name: String,
    /// 默认API URL
    pub default_api_url: String,
    /// 文档链接
    pub documentation_url: Option<String>,
    /// 支持的模型列表
    pub models: Vec<ModelInfo>,
    /// 是否需要API密钥
    pub requires_api_key: bool,
}

/// LLM供应商注册表
pub struct LLMRegistry {
    providers: HashMap<LLMProviderType, ProviderInfo>,
}

impl LLMRegistry {
    /// 创建新的注册表实例
    pub fn new() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
        };
        registry.initialize_default_providers();
        registry
    }

    /// 初始化默认供应商
    fn initialize_default_providers(&mut self) {
        // OpenAI
        self.providers.insert(
            LLMProviderType::OpenAI,
            ProviderInfo {
                provider_type: LLMProviderType::OpenAI,
                display_name: "OpenAI".to_string(),
                default_api_url: "https://api.openai.com/v1".to_string(),
                documentation_url: Some(
                    "https://platform.openai.com/docs/api-reference".to_string(),
                ),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "gpt-4".to_string(),
                        display_name: "GPT-4".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 8192,
                            temperature_range: Some((0.0, 2.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "gpt-4-turbo".to_string(),
                        display_name: "GPT-4 Turbo".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 128000,
                            temperature_range: Some((0.0, 2.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "gpt-3.5-turbo".to_string(),
                        display_name: "GPT-3.5 Turbo".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 16385,
                            temperature_range: Some((0.0, 2.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "o1-preview".to_string(),
                        display_name: "o1-preview".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: false,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: true,
                            max_context_tokens: 32768,
                            temperature_range: None, // o1 models don't support temperature
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "o1-mini".to_string(),
                        display_name: "o1-mini".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: false,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: true,
                            max_context_tokens: 32768,
                            temperature_range: None,
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "o3-mini".to_string(),
                        display_name: "o3-mini".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: false,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: true,
                            max_context_tokens: 32768,
                            temperature_range: None,
                        },
                        deprecated: false,
                    },
                ],
            },
        );

        // Anthropic
        self.providers.insert(
            LLMProviderType::Anthropic,
            ProviderInfo {
                provider_type: LLMProviderType::Anthropic,
                display_name: "Anthropic Claude".to_string(),
                default_api_url: "https://api.anthropic.com/v1".to_string(),
                documentation_url: Some(
                    "https://docs.anthropic.com/claude/reference/getting-started-with-the-api"
                        .to_string(),
                ),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "claude-3-opus-20240229".to_string(),
                        display_name: "Claude 3 Opus".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 200000,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "claude-3-sonnet-20240229".to_string(),
                        display_name: "Claude 3 Sonnet".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 200000,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "claude-3-haiku-20240307".to_string(),
                        display_name: "Claude 3 Haiku".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 200000,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                ],
            },
        );

        // Google Gemini
        self.providers.insert(
            LLMProviderType::Gemini,
            ProviderInfo {
                provider_type: LLMProviderType::Gemini,
                display_name: "Google Gemini".to_string(),
                default_api_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                documentation_url: Some("https://ai.google.dev/docs".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "gemini-pro".to_string(),
                        display_name: "Gemini Pro".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 32768,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "gemini-pro-vision".to_string(),
                        display_name: "Gemini Pro Vision".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 32768,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "gemini-ultra".to_string(),
                        display_name: "Gemini Ultra".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 32768,
                            temperature_range: Some((0.0, 1.0)),
                        },
                        deprecated: false,
                    },
                ],
            },
        );

        // Qwen
        self.providers.insert(
            LLMProviderType::Qwen,
            ProviderInfo {
                provider_type: LLMProviderType::Qwen,
                display_name: "Qwen".to_string(),
                default_api_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                documentation_url: Some(
                    "https://help.aliyun.com/document_detail/2712581.html".to_string(),
                ),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "qwen-max".to_string(),
                        display_name: "Qwen Max".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: true,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 32768,
                            temperature_range: Some((0.1, 2.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "qwen-plus".to_string(),
                        display_name: "Qwen Plus".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 32768,
                            temperature_range: Some((0.1, 2.0)),
                        },
                        deprecated: false,
                    },
                    ModelInfo {
                        id: "qwen-turbo".to_string(),
                        display_name: "Qwen Turbo".to_string(),
                        capabilities: ModelCapabilities {
                            supports_tools: true,
                            supports_vision: false,
                            supports_streaming: true,
                            is_reasoning_model: false,
                            max_context_tokens: 8192,
                            temperature_range: Some((0.1, 2.0)),
                        },
                        deprecated: false,
                    },
                ],
            },
        );
    }

    /// 获取所有供应商
    pub fn get_all_providers(&self) -> Vec<&ProviderInfo> {
        self.providers.values().collect()
    }

    /// 根据类型获取供应商信息
    pub fn get_provider(&self, provider_type: &LLMProviderType) -> Option<&ProviderInfo> {
        self.providers.get(provider_type)
    }

    /// 获取供应商的模型列表
    pub fn get_models_for_provider(&self, provider_type: &LLMProviderType) -> Vec<&ModelInfo> {
        self.providers
            .get(provider_type)
            .map(|provider| provider.models.iter().collect())
            .unwrap_or_default()
    }

    /// 根据模型ID查找供应商和模型信息
    pub fn find_model(&self, model_id: &str) -> Option<(&ProviderInfo, &ModelInfo)> {
        for provider in self.providers.values() {
            for model in &provider.models {
                if model.id == model_id {
                    return Some((provider, model));
                }
            }
        }
        None
    }

    /// 检查模型是否支持特定功能
    pub fn model_supports_feature(&self, model_id: &str, feature: &str) -> bool {
        if let Some((_, model)) = self.find_model(model_id) {
            match feature {
                "tools" => model.capabilities.supports_tools,
                "vision" => model.capabilities.supports_vision,
                "streaming" => model.capabilities.supports_streaming,
                "reasoning" => model.capabilities.is_reasoning_model,
                _ => false,
            }
        } else {
            false
        }
    }

    /// 获取模型的最大上下文长度
    pub fn get_model_max_context(&self, model_id: &str) -> Option<u32> {
        if let Some((_, model)) = self.find_model(model_id) {
            Some(model.capabilities.max_context_tokens)
        } else {
            None
        }
    }

    /// 验证模型是否存在且可用
    pub fn is_model_available(&self, model_id: &str) -> bool {
        if let Some((_, model)) = self.find_model(model_id) {
            !model.deprecated
        } else {
            false
        }
    }
}

impl Default for LLMRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = LLMRegistry::new();
        assert!(!registry.get_all_providers().is_empty());
    }

    #[test]
    fn test_find_model() {
        let registry = LLMRegistry::new();
        let result = registry.find_model("gpt-4");
        assert!(result.is_some());

        let (provider, model) = result.unwrap();
        assert_eq!(provider.provider_type, LLMProviderType::OpenAI);
        assert_eq!(model.id, "gpt-4");
    }

    #[test]
    fn test_model_capabilities() {
        let registry = LLMRegistry::new();

        // Test o1 model (reasoning model)
        assert!(registry.model_supports_feature("o1-preview", "reasoning"));
        assert!(!registry.model_supports_feature("o1-preview", "tools"));

        // Test GPT-4 (regular model)
        assert!(registry.model_supports_feature("gpt-4", "tools"));
        assert!(registry.model_supports_feature("gpt-4", "vision"));
        assert!(!registry.model_supports_feature("gpt-4", "reasoning"));
    }

    #[test]
    fn test_model_availability() {
        let registry = LLMRegistry::new();
        assert!(registry.is_model_available("gpt-4"));
        assert!(!registry.is_model_available("non-existent-model"));
    }
}
