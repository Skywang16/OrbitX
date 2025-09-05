use crate::llm::{
    providers::{
        anthropic::AnthropicProvider, base::LLMProvider, gemini::GeminiProvider,
        openai::OpenAIProvider,
    },
    types::{LLMProviderConfig, LLMProviderType, LLMResult},
};

/// Provider 工厂
///
/// 根据提供的配置，创建并返回一个具体的 LLMProvider 实例。
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_provider(config: LLMProviderConfig) -> LLMResult<Box<dyn LLMProvider>> {
        match config.provider_type {
            LLMProviderType::OpenAI | LLMProviderType::Custom | LLMProviderType::Qwen => {
                // OpenAI, Custom, 和 Qwen (兼容OpenAI) 都使用 OpenAIProvider
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
