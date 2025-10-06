use crate::ai::types::{AIModelConfig, AIProvider, ModelType};
use crate::storage::repositories::{Repository, RepositoryManager};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct AIService {
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AIModelUpdatePayload {
    name: Option<String>,
    provider: Option<AIProvider>,
    api_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    model_type: Option<ModelType>,
    enabled: Option<bool>,
    options: Option<Value>,
}

struct ProviderHttpRequest {
    provider_label: &'static str,
    url: String,
    headers: HeaderMap,
    payload: Value,
    timeout: Duration,
    tolerated: &'static [StatusCode],
}

enum ConnectionProbe {
    Http(ProviderHttpRequest),
    Gemini,
}

impl AIService {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    pub async fn initialize(&self) -> AppResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> AppResult<Vec<AIModelConfig>> {
        self.repositories
            .ai_models()
            .find_all()
            .await
            .context("加载AI模型配置失败")
    }

    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        self.repositories
            .ai_models()
            .save(&config)
            .await
            .map(|_| ())
            .context("保存AI模型配置失败")
    }

    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        self.repositories
            .ai_models()
            .delete_by_string_id(model_id)
            .await
            .with_context(|| format!("删除AI模型配置失败: {}", model_id))
    }

    pub async fn update_model(&self, model_id: &str, updates: Value) -> AppResult<()> {
        let update_payload: AIModelUpdatePayload =
            serde_json::from_value(updates).context("解析AI模型更新请求失败")?;

        let repo = self.repositories.ai_models();
        let mut existing = repo
            .find_by_string_id(model_id)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;

        if let Some(name) = update_payload.name.and_then(trimmed) {
            existing.name = name;
        }
        if let Some(provider) = update_payload.provider {
            existing.provider = provider;
        }
        if let Some(url) = update_payload.api_url.and_then(trimmed) {
            existing.api_url = url;
        }
        if let Some(api_key) = update_payload.api_key {
            existing.api_key = api_key;
        }
        if let Some(model) = update_payload.model.and_then(trimmed) {
            existing.model = model;
        }
        if let Some(model_type) = update_payload.model_type {
            existing.model_type = model_type;
        }
        if let Some(enabled) = update_payload.enabled {
            existing.enabled = enabled;
        }
        if let Some(options) = update_payload.options {
            existing.options = Some(options);
        }

        existing.updated_at = Utc::now();

        repo.update(&existing).await.context("更新AI模型配置失败")
    }

    pub async fn test_connection(&self, model_id: &str) -> AppResult<String> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await?
            .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;

        self.test_connection_with_config(&model).await
    }

    pub async fn test_connection_with_config(&self, model: &AIModelConfig) -> AppResult<String> {
        let probe = self.build_probe(model)?;

        match probe {
            ConnectionProbe::Http(request) => self.execute_http_probe(request).await,
            ConnectionProbe::Gemini => self.execute_gemini_probe(model).await,
        }
    }

    fn build_probe(&self, model: &AIModelConfig) -> AppResult<ConnectionProbe> {
        let timeout = self.resolve_timeout(model);

        match model.provider {
            AIProvider::OpenAI => {
                let url = join_url(model.api_url.trim(), "v1/chat/completions");
                let headers =
                    header_map(&[("authorization", format!("Bearer {}", model.api_key))])?;
                let payload = basic_chat_payload(&model.model);
                Ok(ConnectionProbe::Http(ProviderHttpRequest {
                    provider_label: "OpenAI",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_STANDARD_CODES,
                }))
            }
            AIProvider::Claude => {
                let url = join_url(model.api_url.trim(), "v1/messages");
                let headers = header_map(&[
                    ("x-api-key", model.api_key.clone()),
                    ("anthropic-version", "2023-06-01".to_string()),
                ])?;
                let payload = json!({
                    "model": model.model,
                    "max_tokens": 1,
                    "messages": [{"role": "user", "content": "Hello"}]
                });
                Ok(ConnectionProbe::Http(ProviderHttpRequest {
                    provider_label: "Claude",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_STANDARD_CODES,
                }))
            }
            AIProvider::Qwen => {
                let url = join_url(model.api_url.trim(), "v1/chat/completions");
                let headers =
                    header_map(&[("authorization", format!("Bearer {}", model.api_key))])?;
                let payload = basic_chat_payload(&model.model);
                Ok(ConnectionProbe::Http(ProviderHttpRequest {
                    provider_label: "Qwen",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_STANDARD_CODES,
                }))
            }
            AIProvider::Custom => {
                let url = join_url(model.api_url.trim(), "v1/chat/completions");
                let headers =
                    header_map(&[("authorization", format!("Bearer {}", model.api_key))])?;
                let payload = basic_chat_payload(&model.model);
                Ok(ConnectionProbe::Http(ProviderHttpRequest {
                    provider_label: "自定义",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_CUSTOM_CODES,
                }))
            }
            AIProvider::Gemini => Ok(ConnectionProbe::Gemini),
        }
    }

    async fn execute_http_probe(&self, request: ProviderHttpRequest) -> AppResult<String> {
        let client = Client::builder()
            .timeout(request.timeout)
            .build()
            .context("创建HTTP客户端失败")?;

        let mut headers = request.headers.clone();
        headers
            .entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));

        debug!("开始{}连接测试: {}", request.provider_label, request.url);

        let response = client
            .post(&request.url)
            .headers(headers)
            .json(&request.payload)
            .send()
            .await
            .with_context(|| format!("{} 请求发送失败", request.provider_label))?;

        let status = response.status();
        if status.is_success() || request.tolerated.iter().any(|code| *code == status) {
            info!("{} 连接测试成功，状态码 {}", request.provider_label, status);
            Ok("连接成功".to_string())
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "(无法读取响应内容)".to_string());
            let error_msg = format!(
                "{} API 错误: {} - {}",
                request.provider_label, status, error_text
            );
            warn!("{}", error_msg);
            Err(anyhow!(error_msg))
        }
    }

    async fn execute_gemini_probe(&self, model: &AIModelConfig) -> AppResult<String> {
        use crate::llm::providers::base::LLMProvider;
        use crate::llm::providers::gemini::GeminiProvider;
        use crate::llm::types::{
            LLMMessage, LLMMessageContent, LLMProviderConfig, LLMProviderType, LLMRequest,
        };

        let provider_options = match model.options.clone() {
            Some(Value::Object(map)) => Some(map.into_iter().collect::<HashMap<_, _>>()),
            _ => None,
        };

        let config = LLMProviderConfig {
            provider_type: LLMProviderType::Gemini,
            api_url: trimmed(model.api_url.clone()),
            api_key: model.api_key.clone(),
            model: model.model.clone(),
            options: provider_options,
        };

        let provider = GeminiProvider::new(config);
        let request = LLMRequest {
            model: model.model.clone(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: LLMMessageContent::Text("Hello".to_string()),
            }],
            temperature: Some(0.0),
            max_tokens: Some(1),
            stream: false,
            tools: None,
            tool_choice: None,
        };

        provider
            .call(request)
            .await
            .map(|_| "连接成功".to_string())
            .map_err(|e| anyhow!("Gemini API 错误: {}", e))
    }

    fn resolve_timeout(&self, model: &AIModelConfig) -> Duration {
        model
            .options
            .as_ref()
            .and_then(|opts| opts.get("timeoutSeconds"))
            .and_then(Value::as_u64)
            .map(|secs| secs.clamp(1, 60))
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(12))
    }
}

fn header_map(entries: &[(&'static str, String)]) -> AppResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in entries {
        let header_name = HeaderName::from_static(name);
        headers.insert(header_name, HeaderValue::from_str(value.trim())?);
    }
    Ok(headers)
}

fn trimmed<S: Into<String>>(value: S) -> Option<String> {
    let s = value.into().trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn basic_chat_payload(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 1,
        "temperature": 0,
    })
}

fn join_url(base: &str, suffix: &str) -> String {
    let base = base.trim_end_matches('/');
    let suffix = suffix.trim_start_matches('/');

    if base.ends_with("/v1") && suffix.starts_with("v1/") {
        format!("{}/{}", base, &suffix[3..])
    } else {
        format!("{}/{}", base, suffix)
    }
}

const TOLERATED_STANDARD_CODES: &[StatusCode] = &[
    StatusCode::BAD_REQUEST,
    StatusCode::UNAUTHORIZED,
    StatusCode::TOO_MANY_REQUESTS,
];

const TOLERATED_CUSTOM_CODES: &[StatusCode] = &[
    StatusCode::BAD_REQUEST,
    StatusCode::UNAUTHORIZED,
    StatusCode::TOO_MANY_REQUESTS,
    StatusCode::UNPROCESSABLE_ENTITY,
];
