use super::pkce::{generate_pkce, generate_state};
use super::provider_trait::OAuthProvider;
use super::providers::OpenAiCodexProvider;
use super::server::OAuthCallbackServer;
use super::types::{OAuthError, OAuthFlowInfo, OAuthResult, PkceCodes};
use crate::storage::database::DatabaseManager;
use crate::storage::repositories::ai_models::{
    OAuthConfig as StorageOAuthConfig, OAuthProvider as StorageOAuthProvider,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info};

/// 等待中的 OAuth 流程
struct PendingFlow {
    receiver: oneshot::Receiver<OAuthResult<(String, PkceCodes)>>,
}

/// OAuth Manager - 管理多个 provider 和协调授权流程
pub struct OAuthManager {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
    callback_server: Arc<Mutex<OAuthCallbackServer>>,
    pending_flows: Mutex<HashMap<String, PendingFlow>>,
    #[allow(dead_code)]
    db: Arc<DatabaseManager>,
}

impl OAuthManager {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        let mut providers: HashMap<String, Box<dyn OAuthProvider>> = HashMap::new();

        // 注册 OpenAI Codex Provider
        let openai_provider = OpenAiCodexProvider::new();
        providers.insert(
            openai_provider.provider_id().to_string(),
            Box::new(openai_provider),
        );

        // 未来可以在这里添加更多 provider:
        // providers.insert("claude_pro".to_string(), Box::new(ClaudeProProvider::new()));
        // providers.insert("gemini_advanced".to_string(), Box::new(GeminiProvider::new()));

        let callback_server = OAuthCallbackServer::new();

        Self {
            providers,
            callback_server,
            pending_flows: Mutex::new(HashMap::new()),
            db,
        }
    }

    /// 启动 OAuth 流程
    pub async fn start_oauth_flow(&self, provider_type: &str) -> OAuthResult<OAuthFlowInfo> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        // 生成 PKCE 和 state
        let pkce = generate_pkce()?;
        let state = generate_state()?;

        // 生成授权 URL
        let authorize_url = provider.generate_authorize_url(&pkce, &state)?;

        // 注册等待回调并获取 receiver
        let mut server = self.callback_server.lock().await;
        let receiver = server.register_flow(state.clone(), pkce.clone()).await;
        drop(server);

        // 保存 pending flow
        let pending = PendingFlow { receiver };
        self.pending_flows
            .lock()
            .await
            .insert(state.clone(), pending);

        info!("OAuth flow started for provider: {}", provider_type);

        Ok(OAuthFlowInfo {
            flow_id: state,
            authorize_url,
            provider: provider_type.to_string(),
        })
    }

    /// 等待 OAuth 回调
    pub async fn wait_for_callback(
        &self,
        flow_id: &str,
        provider_type: &str,
    ) -> OAuthResult<StorageOAuthConfig> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        // 取出 pending flow
        let pending = self
            .pending_flows
            .lock()
            .await
            .remove(flow_id)
            .ok_or_else(|| OAuthError::FlowNotFound(flow_id.to_string()))?;

        // 等待回调 (带超时)
        let timeout = tokio::time::Duration::from_secs(5 * 60); // 5 分钟超时

        let result = tokio::time::timeout(timeout, pending.receiver)
            .await
            .map_err(|_| OAuthError::Timeout)?
            .map_err(|_| OAuthError::Other("Channel closed".to_string()))??;

        let (code, pkce) = result;

        // 用授权码换取 tokens
        debug!("Exchanging code for tokens");
        let tokens = provider.exchange_code_for_tokens(&code, &pkce).await?;

        // 提取 metadata
        let metadata = provider.extract_metadata(&tokens)?;

        // 计算过期时间
        let expires_at = tokens
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        // 转换为存储格式
        let storage_provider = match provider_type {
            "openai_codex" => StorageOAuthProvider::OpenAiCodex,
            "claude_pro" => StorageOAuthProvider::ClaudePro,
            "gemini_advanced" => StorageOAuthProvider::GeminiAdvanced,
            _ => return Err(OAuthError::InvalidProvider(provider_type.to_string())),
        };

        Ok(StorageOAuthConfig {
            provider: storage_provider,
            refresh_token: tokens.refresh_token,
            access_token: Some(tokens.access_token),
            expires_at,
            metadata: Some(metadata),
        })
    }

    /// 取消 OAuth 流程
    pub async fn cancel_flow(&self, flow_id: &str) -> OAuthResult<()> {
        // 移除 pending flow
        self.pending_flows.lock().await.remove(flow_id);

        // 同时从 callback server 中移除
        let mut server = self.callback_server.lock().await;
        server.cancel_flow(flow_id).await;

        info!("OAuth flow cancelled: {}", flow_id);
        Ok(())
    }

    /// 刷新 token
    pub async fn refresh_token(&self, oauth_config: &mut StorageOAuthConfig) -> OAuthResult<()> {
        let provider_id = oauth_config.provider.to_string();

        let provider = self
            .providers
            .get(&provider_id)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_id.clone()))?;

        debug!("Refreshing token for provider: {}", provider_id);

        let tokens = provider
            .refresh_access_token(&oauth_config.refresh_token)
            .await?;

        // 提取 metadata 先，避免 borrow 问题
        let metadata = provider.extract_metadata(&tokens)?;

        // 更新配置
        oauth_config.access_token = Some(tokens.access_token);
        oauth_config.refresh_token = tokens.refresh_token;
        oauth_config.expires_at = tokens
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);
        oauth_config.metadata = Some(metadata);

        info!("Token refreshed for provider: {}", provider_id);
        Ok(())
    }

    /// 获取 provider
    pub fn get_provider(&self, provider_type: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(provider_type).map(|p| p.as_ref())
    }

    /// 检查 token 是否需要刷新
    pub fn should_refresh_token(&self, oauth_config: &StorageOAuthConfig) -> bool {
        if let Some(expires_at) = oauth_config.expires_at {
            let provider_id = oauth_config.provider.to_string();
            if let Some(provider) = self.providers.get(&provider_id) {
                return provider.should_refresh_token(expires_at);
            }
        }
        false
    }
}
