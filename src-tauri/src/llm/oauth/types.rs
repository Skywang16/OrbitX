use serde::{Deserialize, Serialize};

/// PKCE 代码对
#[derive(Debug, Clone)]
pub struct PkceCodes {
    pub verifier: String,
    pub challenge: String,
}

/// Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<u64>,
}

/// OAuth 流程句柄
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlowInfo {
    pub flow_id: String,
    pub authorize_url: String,
    pub provider: String,
}

/// OAuth 错误
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("OAuth flow not found: {0}")]
    FlowNotFound(String),

    #[error("OAuth callback timeout")]
    Timeout,

    #[error("OAuth cancelled by user")]
    Cancelled,

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Other error: {0}")]
    Other(String),
}

pub type OAuthResult<T> = Result<T, OAuthError>;
