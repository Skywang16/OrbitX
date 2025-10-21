use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult},
    preset_models::PresetModel,
    providers::base::LLMProvider,
    types::LLMProviderConfig,
};

type ProviderBuilder = fn(LLMProviderConfig) -> Box<dyn LLMProvider>;

/// Provider 元数据信息（给前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    pub provider_type: String,
    pub display_name: String,
    pub default_api_url: String,
    pub preset_models: Vec<PresetModel>,
}

struct ProviderEntry {
    builder: ProviderBuilder,
    metadata: ProviderMetadata,
}

pub struct ProviderRegistry {
    providers: HashMap<String, ProviderEntry>,
}

impl ProviderRegistry {
    fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn global() -> &'static ProviderRegistry {
        static REGISTRY: OnceLock<ProviderRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = Self::new();
            registry.register_builtin();
            registry
        })
    }

    fn register_builtin(&mut self) {
        use crate::llm::{preset_models::*, providers::*};

        self.register(
            "anthropic",
            |config| Box::new(AnthropicProvider::new(config)),
            ProviderMetadata {
                provider_type: "anthropic".to_string(),
                display_name: "Anthropic".to_string(),
                default_api_url: "https://api.anthropic.com/v1".to_string(),
                preset_models: ANTHROPIC_MODELS.clone(),
            },
        );

        self.register(
            "openai_compatible",
            |config| Box::new(OpenAIProvider::new(config)),
            ProviderMetadata {
                provider_type: "openai_compatible".to_string(),
                display_name: "OpenAI Compatible".to_string(),
                default_api_url: "".to_string(),
                // OpenAI Compatible 不提供预设模型，用户需要自己输入
                preset_models: vec![],
            },
        );

        self.register(
            "openai",
            |config| Box::new(OpenAIProvider::new(config)),
            ProviderMetadata {
                provider_type: "openai".to_string(),
                display_name: "OpenAI".to_string(),
                default_api_url: "https://api.openai.com/v1".to_string(),
                preset_models: OPENAI_MODELS.clone(),
            },
        );

        self.register(
            "gemini",
            |config| Box::new(GeminiProvider::new(config)),
            ProviderMetadata {
                provider_type: "gemini".to_string(),
                display_name: "Gemini".to_string(),
                default_api_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                preset_models: GEMINI_MODELS.clone(),
            },
        );
    }

    fn register(&mut self, name: &str, builder: ProviderBuilder, metadata: ProviderMetadata) {
        self.providers
            .insert(name.to_string(), ProviderEntry { builder, metadata });
    }

    pub fn create(&self, config: LLMProviderConfig) -> LlmProviderResult<Box<dyn LLMProvider>> {
        let entry = self.providers.get(&config.provider_type).ok_or_else(|| {
            LlmProviderError::UnsupportedProvider {
                provider: config.provider_type.clone(),
            }
        })?;

        Ok((entry.builder)(config))
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_provider_metadata(&self, provider_type: &str) -> Option<&ProviderMetadata> {
        self.providers.get(provider_type).map(|e| &e.metadata)
    }

    pub fn get_all_providers_metadata(&self) -> Vec<&ProviderMetadata> {
        self.providers.values().map(|e| &e.metadata).collect()
    }

    pub fn supports(&self, provider_type: &str) -> bool {
        self.providers.contains_key(provider_type)
    }
}
