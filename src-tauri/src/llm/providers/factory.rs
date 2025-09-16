use crate::llm::{
    providers::{
        anthropic::AnthropicProvider, base::LLMProvider, gemini::GeminiProvider,
        openai::OpenAIProvider,
    },
    types::{LLMProviderConfig, LLMProviderType},
};
use anyhow::Result;

/// LLM提供者工厂
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_provider(config: LLMProviderConfig) -> Result<Box<dyn LLMProvider>> {
        match config.provider_type {
            LLMProviderType::OpenAI | LLMProviderType::Custom | LLMProviderType::Qwen => {
                let provider = OpenAIProvider::new(config);
                Ok(Box::new(provider))
            }
            LLMProviderType::Anthropic => {
                let provider = AnthropicProvider::new(config);
                Ok(Box::new(provider))
            }
            LLMProviderType::Gemini => {
                let provider = GeminiProvider::new(config);
                Ok(Box::new(provider))
            }
        }
    }
}
