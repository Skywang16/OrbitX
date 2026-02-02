use crate::llm::oauth::provider_trait::OAuthProvider;
use crate::llm::oauth::server::OAuthCallbackServer;
use crate::llm::oauth::types::{OAuthError, OAuthResult, PkceCodes, TokenResponse};
use crate::storage::repositories::ai_models::OAuthConfig;
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::RequestBuilder;
use serde_json::{json, Value};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";

/// OpenAI Codex OAuth Provider
pub struct OpenAiCodexProvider {
    client: reqwest::Client,
}

impl OpenAiCodexProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// 从 JWT 中提取 account_id
    fn extract_account_id_from_jwt(token: &str) -> Option<String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // 解码 payload (base64url)
        let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
        let claims: Value = serde_json::from_slice(&payload).ok()?;

        // 尝试从多个字段提取
        claims
            .get("chatgpt_account_id")
            .or_else(|| {
                claims
                    .get("https://api.openai.com/auth")
                    .and_then(|v| v.get("chatgpt_account_id"))
            })
            .or_else(|| {
                claims
                    .get("organizations")
                    .and_then(|orgs| orgs.as_array()?.first())
                    .and_then(|org| org.get("id"))
            })
            .and_then(|v| v.as_str())
            .map(String::from)
    }
}

#[async_trait]
impl OAuthProvider for OpenAiCodexProvider {
    fn provider_id(&self) -> &str {
        "openai_codex"
    }

    fn display_name(&self) -> &str {
        "OpenAI ChatGPT Plus/Pro (Codex)"
    }

    fn generate_authorize_url(&self, pkce: &PkceCodes, state: &str) -> OAuthResult<String> {
        let callback_url = OAuthCallbackServer::callback_url();
        
        let params = [
            ("response_type", "code"),
            ("client_id", CLIENT_ID),
            ("redirect_uri", &callback_url),
            ("scope", "openid profile email offline_access"),
            ("code_challenge", &pkce.challenge),
            ("code_challenge_method", "S256"),
            ("id_token_add_organizations", "true"),
            ("state", state),
        ];

        let query = serde_urlencoded::to_string(&params)
            .map_err(|e| OAuthError::Other(format!("Failed to encode query: {}", e)))?;

        Ok(format!("{}?{}", AUTH_ENDPOINT, query))
    }

    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        pkce: &PkceCodes,
    ) -> OAuthResult<TokenResponse> {
        let callback_url = OAuthCallbackServer::callback_url();
        
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &callback_url),
            ("client_id", CLIENT_ID),
            ("code_verifier", &pkce.verifier),
        ];

        let response = self
            .client
            .post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchange(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    async fn refresh_access_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ];

        let response = self
            .client
            .post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenRefresh(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    fn extract_metadata(&self, tokens: &TokenResponse) -> OAuthResult<Value> {
        let mut metadata = json!({});

        // 尝试从 id_token 提取 account_id
        if let Some(id_token) = &tokens.id_token {
            if let Some(account_id) = Self::extract_account_id_from_jwt(id_token) {
                metadata["account_id"] = json!(account_id);
            }
        }

        // 如果没有 id_token,尝试从 access_token 提取
        if metadata.get("account_id").is_none() {
            if let Some(account_id) = Self::extract_account_id_from_jwt(&tokens.access_token) {
                metadata["account_id"] = json!(account_id);
            }
        }

        Ok(metadata)
    }

    async fn prepare_request(
        &self,
        mut request: RequestBuilder,
        oauth_config: &OAuthConfig,
    ) -> OAuthResult<RequestBuilder> {
        // 添加 Bearer token
        if let Some(access_token) = &oauth_config.access_token {
            request = request.header("Authorization", format!("Bearer {}", access_token));
        }

        // 添加 ChatGPT-Account-Id (用于组织订阅)
        if let Some(metadata) = &oauth_config.metadata {
            if let Some(account_id) = metadata.get("account_id") {
                if let Some(account_id_str) = account_id.as_str() {
                    request = request.header("ChatGPT-Account-Id", account_id_str);
                }
            }
        }

        // 添加 originator
        request = request.header("originator", "OrbitX");
        request = request.header("Content-Type", "application/json");

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let provider = OpenAiCodexProvider::new();
        assert_eq!(provider.provider_id(), "openai_codex");
        assert!(!provider.display_name().is_empty());
    }
}
