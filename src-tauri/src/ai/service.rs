use crate::ai::error::{AIServiceError, AIServiceResult};
use crate::ai::types::{AIModelConfig, AIProvider, ModelType};
use crate::storage::repositories::{Repository, RepositoryManager};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
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
    provider: Option<AIProvider>,
    api_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    model_type: Option<ModelType>,
    options: Option<Value>,
    use_custom_base_url: Option<bool>,
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
}

impl AIService {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    pub async fn initialize(&self) -> AIServiceResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> AIServiceResult<Vec<AIModelConfig>> {
        self.repositories
            .ai_models()
            .find_all()
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_all",
                source: err,
            })
    }

    pub async fn add_model(&self, config: AIModelConfig) -> AIServiceResult<()> {
        self.repositories
            .ai_models()
            .save(&config)
            .await
            .map(|_| ())
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.save",
                source: err,
            })
    }

    pub async fn remove_model(&self, model_id: &str) -> AIServiceResult<()> {
        self.repositories
            .ai_models()
            .delete_by_string_id(model_id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.delete_by_string_id",
                source: err,
            })
    }

    pub async fn update_model(&self, model_id: &str, updates: Value) -> AIServiceResult<()> {
        let update_payload: AIModelUpdatePayload =
            serde_json::from_value(updates).map_err(AIServiceError::InvalidUpdatePayload)?;

        let repo = self.repositories.ai_models();
        let mut existing = repo
            .find_by_string_id(model_id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_by_string_id",
                source: err,
            })?
            .ok_or_else(|| AIServiceError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;

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
        if let Some(options) = update_payload.options {
            existing.options = Some(options);
        }
        if let Some(use_custom_base_url) = update_payload.use_custom_base_url {
            existing.use_custom_base_url = Some(use_custom_base_url);
        }

        existing.updated_at = Utc::now();

        repo.update(&existing)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.update",
                source: err,
            })
    }

    pub async fn test_connection(&self, model_id: &str) -> AIServiceResult<String> {
        let model = self
            .repositories
            .ai_models()
            .find_by_string_id(model_id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_by_string_id",
                source: err,
            })?
            .ok_or_else(|| AIServiceError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;

        self.test_connection_with_config(&model).await
    }

    pub async fn test_connection_with_config(
        &self,
        model: &AIModelConfig,
    ) -> AIServiceResult<String> {
        let probe = self.build_probe(model)?;

        match probe {
            ConnectionProbe::Http(request) => self.execute_http_probe(request).await,
        }
    }

    fn build_probe(&self, model: &AIModelConfig) -> AIServiceResult<ConnectionProbe> {
        let timeout = self.resolve_timeout(model);

        match model.provider {
            AIProvider::Anthropic => {
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
                    provider_label: "Anthropic",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_STANDARD_CODES,
                }))
            }
            AIProvider::OpenAiCompatible => {
                let url = join_url(model.api_url.trim(), "v1/chat/completions");
                let headers =
                    header_map(&[("authorization", format!("Bearer {}", model.api_key))])?;
                let payload = basic_chat_payload(&model.model);
                Ok(ConnectionProbe::Http(ProviderHttpRequest {
                    provider_label: "OpenAI Compatible",
                    url,
                    headers,
                    payload,
                    timeout,
                    tolerated: &TOLERATED_CUSTOM_CODES,
                }))
            }
        }
    }

    async fn execute_http_probe(&self, request: ProviderHttpRequest) -> AIServiceResult<String> {
        let client = Client::builder()
            .timeout(request.timeout)
            .build()
            .map_err(AIServiceError::HttpClient)?;

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
            .map_err(|err| AIServiceError::ProviderRequest {
                provider: request.provider_label,
                source: err,
            })?;

        let status = response.status();

        // 成功状态码：2xx
        if status.is_success() {
            info!(
                "{} connection test successful, status code {}",
                request.provider_label, status
            );
            return Ok("Connection successful".to_string());
        }

        // 认证失败：401/403 - 这是明确的错误，不应该被容忍
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unauthorized".to_string());
            warn!(
                "{} authentication failed: {}",
                request.provider_label, status
            );
            return Err(AIServiceError::ProviderApi {
                provider: request.provider_label,
                status,
                message: format!("Authentication failed: {}", error_text),
            });
        }

        // 可容忍的状态码：说明 API 端点可用且认证有效
        // 400: 请求格式错误，但说明服务器可达
        // 429: 请求过多，但说明认证成功
        if request.tolerated.iter().any(|code| *code == status) {
            info!(
                "{} connection test successful (tolerated status: {})",
                request.provider_label, status
            );
            return Ok("Connection successful".to_string());
        }

        // 其他错误状态码
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "(failed to read response body)".to_string());
        let error_msg = format!(
            "{} API error: {} - {}",
            request.provider_label, status, error_text
        );
        warn!("{}", error_msg);
        Err(AIServiceError::ProviderApi {
            provider: request.provider_label,
            status,
            message: error_msg,
        })
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

fn header_map(entries: &[(&'static str, String)]) -> AIServiceResult<HeaderMap> {
    // 预分配容量，避免多次rehash
    let mut headers = HeaderMap::with_capacity(entries.len());
    for (name, value) in entries {
        let header_name = HeaderName::from_static(name);
        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|err| AIServiceError::InvalidHeaderValue { name, source: err })?;
        headers.insert(header_name, header_value);
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

const TOLERATED_STANDARD_CODES: &[StatusCode] =
    &[StatusCode::BAD_REQUEST, StatusCode::TOO_MANY_REQUESTS];

const TOLERATED_CUSTOM_CODES: &[StatusCode] = &[
    StatusCode::BAD_REQUEST,
    StatusCode::TOO_MANY_REQUESTS,
    StatusCode::UNPROCESSABLE_ENTITY,
];
