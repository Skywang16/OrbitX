use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult},
    preset_models::PresetModel,
    providers::{AnthropicProvider, GeminiProvider, OpenAIProvider, Provider},
    types::LLMProviderConfig,
};

/// Provider 元数据 - 编译期常量
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    pub provider_type: &'static str,
    pub display_name: &'static str,
    pub default_api_url: &'static str,
    pub preset_models: Vec<PresetModel>,
}

/// 全局 Provider 元数据 - 延迟初始化，但只分配一次
static PROVIDER_METADATA: Lazy<Vec<ProviderMetadata>> = Lazy::new(|| {
    use crate::llm::preset_models::*;

    vec![
        ProviderMetadata {
            provider_type: "anthropic",
            display_name: "Anthropic",
            default_api_url: "https://api.anthropic.com/v1",
            preset_models: ANTHROPIC_MODELS.clone(),
        },
        ProviderMetadata {
            provider_type: "openai",
            display_name: "OpenAI",
            default_api_url: "https://api.openai.com/v1",
            preset_models: OPENAI_MODELS.clone(),
        },
        ProviderMetadata {
            provider_type: "openai_compatible",
            display_name: "OpenAI Compatible",
            default_api_url: "",
            preset_models: vec![],
        },
        ProviderMetadata {
            provider_type: "gemini",
            display_name: "Gemini",
            default_api_url: "https://generativelanguage.googleapis.com/v1beta",
            preset_models: GEMINI_MODELS.clone(),
        },
    ]
});

/// Provider 注册表 - 零成本抽象版本
///
/// 删除所有运行时开销：
/// - 零哈希查找（编译期 match）
/// - 零堆分配（直接构造枚举）
/// - 零函数指针（静态分发）
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn global() -> &'static ProviderRegistry {
        &ProviderRegistry
    }

    /// 创建 Provider - 编译期 match，零运行时开销
    #[inline]
    pub fn create(&self, config: LLMProviderConfig) -> LlmProviderResult<Provider> {
        let provider_type = config.provider_type.as_str();

        match provider_type {
            "openai" | "openai_compatible" => Ok(Provider::OpenAI(OpenAIProvider::new(config))),
            "anthropic" => Ok(Provider::Anthropic(AnthropicProvider::new(config))),
            "gemini" => Ok(Provider::Gemini(GeminiProvider::new(config))),
            _ => Err(LlmProviderError::UnsupportedProvider {
                provider: provider_type.to_string(),
            }),
        }
    }

    /// 支持的 provider 列表 - 编译期常量
    #[inline]
    pub fn list_providers(&self) -> &'static [&'static str] {
        &["anthropic", "openai", "openai_compatible", "gemini"]
    }

    /// 获取 Provider 元数据
    pub fn get_provider_metadata(&self, provider_type: &str) -> Option<&ProviderMetadata> {
        PROVIDER_METADATA
            .iter()
            .find(|m| m.provider_type == provider_type)
    }

    /// 获取所有 Provider 元数据
    pub fn get_all_providers_metadata(&self) -> &[ProviderMetadata] {
        &PROVIDER_METADATA
    }

    /// 检查是否支持指定 provider - 编译期 match
    #[inline]
    pub fn supports(&self, provider_type: &str) -> bool {
        matches!(
            provider_type,
            "openai" | "openai_compatible" | "anthropic" | "gemini"
        )
    }
}
