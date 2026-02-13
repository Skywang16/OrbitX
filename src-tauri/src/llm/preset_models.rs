//! 预设模型数据定义
//!
//! 本模块包含所有 LLM 供应商的预设模型列表。

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// 预设模型信息

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetModel {
    /// 模型 ID（用于 API 调用）
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 最大输出 tokens（None 表示无限制或由模型动态决定）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 上下文窗口大小（tokens）
    pub context_window: u32,
    /// 模型描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PresetModel {
    fn new(
        id: &str,
        name: &str,
        max_tokens: Option<u32>,
        context_window: u32,
        description: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            max_tokens,
            context_window,
            description: description.map(|s| s.to_string()),
        }
    }
}

// =============================================================================
// Anthropic Models
// =============================================================================
// https://docs.anthropic.com/en/docs/about-claude/models
pub static ANTHROPIC_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // Claude 4.5 系列
        PresetModel::new(
            "claude-sonnet-4-5-20250929",
            "Claude Sonnet 4.5",
            Some(8192),
            200_000,
            None,
        ),
        // Claude 4 系列
        PresetModel::new(
            "claude-sonnet-4-20250514",
            "Claude Sonnet 4",
            Some(8192),
            200_000,
            None,
        ),
        PresetModel::new(
            "claude-opus-4-1-20250805",
            "Claude Opus 4.1",
            Some(8192),
            200_000,
            None,
        ),
        PresetModel::new(
            "claude-opus-4-20250514",
            "Claude Opus 4",
            Some(8192),
            200_000,
            None,
        ),
        // Claude 3.7 系列
        PresetModel::new(
            "claude-3-7-sonnet-20250219",
            "Claude 3.7 Sonnet",
            Some(8192),
            200_000,
            None,
        ),
        // Claude 3.5 系列
        PresetModel::new(
            "claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet",
            Some(8192),
            200_000,
            None,
        ),
        PresetModel::new(
            "claude-3-5-haiku-20241022",
            "Claude 3.5 Haiku",
            Some(8192),
            200_000,
            None,
        ),
    ]
});

// =============================================================================
// OpenAI Models
// =============================================================================
// https://openai.com/api/pricing/
pub static OPENAI_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // GPT-5 系列
        PresetModel::new("gpt-5-2025-08-07", "GPT-5", Some(8192), 272_000, None),
        PresetModel::new(
            "gpt-5-mini-2025-08-07",
            "GPT-5 Mini",
            Some(8192),
            272_000,
            None,
        ),
        PresetModel::new(
            "gpt-5-nano-2025-08-07",
            "GPT-5 Nano",
            Some(8192),
            272_000,
            None,
        ),
        PresetModel::new(
            "gpt-5-chat-latest",
            "GPT-5 Chat Latest",
            Some(8192),
            400_000,
            None,
        ),
        // o 系列（推理模型）
        PresetModel::new("o3", "o3", Some(100_000), 200_000, None),
        PresetModel::new("o4-mini", "o4 Mini", Some(100_000), 200_000, None),
        // GPT-4.1 系列
        PresetModel::new("gpt-4.1", "GPT-4.1", Some(32_768), 1_047_576, None),
        PresetModel::new(
            "gpt-4.1-mini",
            "GPT-4.1 Mini",
            Some(32_768),
            1_047_576,
            None,
        ),
        PresetModel::new("gpt-4o", "GPT-4o", Some(4096), 128_000, None),
        PresetModel::new("gpt-4o-mini", "GPT-4o Mini", Some(16_384), 128_000, None),
    ]
});

// =============================================================================
// OpenAI Compatible Models (建议列表)
// =============================================================================
// 这些是常见的 OpenAI Compatible 模型，用户可以选择或自定义
pub static OPENAI_COMPATIBLE_SUGGESTIONS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // DeepSeek 模型
        PresetModel::new(
            "deepseek-chat",
            "DeepSeek Chat",
            Some(8192),
            32_000,
            Some("DeepSeek AI - 需要配置 API URL"),
        ),
        PresetModel::new(
            "deepseek-coder",
            "DeepSeek Coder",
            Some(8192),
            16_000,
            Some("DeepSeek Coder - 需要配置 API URL"),
        ),
        // Ollama 本地模型
        PresetModel::new(
            "llama3.1",
            "Llama 3.1 (Ollama)",
            Some(8192),
            128_000,
            Some("需要本地运行 Ollama，默认 URL: http://localhost:11434/v1"),
        ),
        PresetModel::new(
            "qwen2.5",
            "Qwen 2.5 (Ollama)",
            Some(8192),
            128_000,
            Some("需要本地运行 Ollama"),
        ),
        // LM Studio
        PresetModel::new(
            "local-model",
            "Local Model (LM Studio)",
            None,
            128_000,
            Some("需要本地运行 LM Studio，默认 URL: http://localhost:1234/v1"),
        ),
        // OpenRouter
        PresetModel::new(
            "openrouter/auto",
            "OpenRouter Auto",
            None,
            200_000,
            Some("OpenRouter 自动选择最佳模型"),
        ),
        // Together AI
        PresetModel::new(
            "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
            "Llama 3.1 70B (Together AI)",
            Some(8192),
            128_000,
            Some("Together AI - 需要配置 API URL"),
        ),
    ]
});

// =============================================================================
// Google Gemini Models
// =============================================================================
// https://ai.google.dev/gemini-api/docs/models/gemini
pub static GEMINI_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // Gemini 2.5 系列
        PresetModel::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            Some(65_536),
            1_048_576,
            None,
        ),
        PresetModel::new(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            Some(65_536),
            1_048_576,
            None,
        ),
        PresetModel::new(
            "gemini-2.5-flash-lite-preview-06-17",
            "Gemini 2.5 Flash Lite Preview",
            Some(64_000),
            1_000_000,
            Some("Preview version - may not be available in all regions"),
        ),
        // Gemini 2.0 系列
        PresetModel::new(
            "gemini-2.0-flash-001",
            "Gemini 2.0 Flash",
            Some(8192),
            1_048_576,
            None,
        ),
        PresetModel::new(
            "gemini-2.0-flash-lite-preview-02-05",
            "Gemini 2.0 Flash Lite Preview",
            Some(8192),
            1_048_576,
            Some("Preview version - free"),
        ),
        PresetModel::new(
            "gemini-2.0-pro-exp-02-05",
            "Gemini 2.0 Pro Experimental",
            Some(8192),
            2_097_152,
            Some("Experimental version - free"),
        ),
        PresetModel::new(
            "gemini-2.0-flash-thinking-exp-01-21",
            "Gemini 2.0 Flash Thinking Experimental",
            Some(65_536),
            1_048_576,
            Some("Experimental version - free"),
        ),
        // Gemini 1.5 系列
        PresetModel::new(
            "gemini-1.5-flash-002",
            "Gemini 1.5 Flash",
            Some(8192),
            1_048_576,
            None,
        ),
        PresetModel::new(
            "gemini-1.5-pro-002",
            "Gemini 1.5 Pro",
            Some(8192),
            2_097_152,
            None,
        ),
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_models_not_empty() {
        assert!(
            !ANTHROPIC_MODELS.is_empty(),
            "Anthropic models should not be empty"
        );
        assert!(
            ANTHROPIC_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_openai_models_not_empty() {
        assert!(
            !OPENAI_MODELS.is_empty(),
            "OpenAI models should not be empty"
        );
        assert!(
            OPENAI_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_gemini_models_not_empty() {
        assert!(
            !GEMINI_MODELS.is_empty(),
            "Gemini models should not be empty"
        );
        assert!(
            GEMINI_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_openai_compatible_suggestions_not_empty() {
        assert!(
            !OPENAI_COMPATIBLE_SUGGESTIONS.is_empty(),
            "OpenAI Compatible suggestions should not be empty"
        );
        assert!(
            OPENAI_COMPATIBLE_SUGGESTIONS
                .iter()
                .all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_preset_model_serialization() {
        let model = PresetModel::new("test-model", "Test Model", Some(4096), 128_000, None);
        let json = serde_json::to_string(&model).expect("Should serialize to JSON");
        assert!(json.contains("\"id\":\"test-model\""));
        assert!(json.contains("\"contextWindow\":128000")); // camelCase
        assert!(json.contains("\"maxTokens\":4096")); // camelCase
    }
}
