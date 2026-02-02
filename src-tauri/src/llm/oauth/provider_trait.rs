use super::types::{OAuthResult, PkceCodes, TokenResponse};
use crate::storage::repositories::ai_models::OAuthConfig;
use async_trait::async_trait;
use reqwest::RequestBuilder;

/// OAuth Provider 统一接口
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider 标识
    fn provider_id(&self) -> &str;
    
    /// 显示名称
    fn display_name(&self) -> &str;
    
    /// 生成授权 URL
    fn generate_authorize_url(&self, pkce: &PkceCodes, state: &str) -> OAuthResult<String>;
    
    /// 用授权码换取 tokens
    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        pkce: &PkceCodes,
    ) -> OAuthResult<TokenResponse>;
    
    /// 刷新 access token
    async fn refresh_access_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse>;
    
    /// 从 tokens 中提取 metadata
    fn extract_metadata(&self, tokens: &TokenResponse) -> OAuthResult<serde_json::Value>;
    
    /// 准备 API 请求 (添加认证头等)
    async fn prepare_request(
        &self,
        request: RequestBuilder,
        oauth_config: &OAuthConfig,
    ) -> OAuthResult<RequestBuilder>;
    
    /// 检查 token 是否需要刷新
    fn should_refresh_token(&self, expires_at: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        let threshold = 5 * 60; // 提前 5 分钟刷新
        expires_at - now < threshold
    }
}
