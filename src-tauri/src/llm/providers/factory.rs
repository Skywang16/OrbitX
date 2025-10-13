use crate::llm::{
    error::LlmProviderResult,
    providers::{anthropic::AnthropicProvider, base::LLMProvider},
    types::{LLMProviderConfig, LLMProviderType},
};

/// LLM提供者工厂
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_provider(config: LLMProviderConfig) -> LlmProviderResult<Box<dyn LLMProvider>> {
        match config.provider_type {
            LLMProviderType::Anthropic | LLMProviderType::OpenAiCompatible => {
                let provider = AnthropicProvider::new(config);
                Ok(Box::new(provider))
            }
        }
    }
}
