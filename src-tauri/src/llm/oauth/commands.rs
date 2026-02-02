use super::manager::OAuthManager;
use super::types::OAuthFlowInfo;
use crate::storage::repositories::ai_models::OAuthConfig;
use std::sync::Arc;
use tauri::State;

/// 启动 OAuth 流程
#[tauri::command]
pub async fn start_oauth_flow(
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthFlowInfo, String> {
    manager
        .start_oauth_flow(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// 等待 OAuth 回调
#[tauri::command]
pub async fn wait_oauth_callback(
    flow_id: String,
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthConfig, String> {
    manager
        .wait_for_callback(&flow_id, &provider)
        .await
        .map_err(|e| e.to_string())
}

/// 取消 OAuth 流程
#[tauri::command]
pub async fn cancel_oauth_flow(
    flow_id: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<(), String> {
    manager
        .cancel_flow(&flow_id)
        .await
        .map_err(|e| e.to_string())
}

/// 刷新 OAuth token
#[tauri::command]
pub async fn refresh_oauth_token(
    mut oauth_config: OAuthConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthConfig, String> {
    manager
        .refresh_token(&mut oauth_config)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(oauth_config)
}

/// 检查 OAuth 状态
#[tauri::command]
pub async fn check_oauth_status(
    oauth_config: OAuthConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<String, String> {
    // 检查是否有 access_token
    if oauth_config.access_token.is_none() {
        return Ok("not_authorized".to_string());
    }
    
    // 检查 token 是否需要刷新
    if manager.should_refresh_token(&oauth_config) {
        Ok("token_expired".to_string())
    } else {
        Ok("authorized".to_string())
    }
}
