use crate::settings::types::{EffectiveSettings, Settings};
use crate::settings::SettingsManager;
use crate::utils::{ApiResponse, TauriApiResult};
use crate::api_success;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_global_settings(state: State<'_, Arc<SettingsManager>>) -> TauriApiResult<Settings> {
    match state.get_global_settings().await {
        Ok(settings) => Ok(api_success!(settings)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

#[tauri::command]
pub async fn update_global_settings(
    settings: Settings,
    state: State<'_, Arc<SettingsManager>>,
) -> TauriApiResult<crate::utils::EmptyData> {
    match state.update_global_settings(&settings).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

#[tauri::command]
pub async fn get_workspace_settings(
    workspace: String,
    state: State<'_, Arc<SettingsManager>>,
) -> TauriApiResult<Option<Settings>> {
    match state
        .get_workspace_settings(PathBuf::from(&workspace))
        .await
    {
        Ok(settings) => Ok(api_success!(settings)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

#[tauri::command]
pub async fn update_workspace_settings(
    workspace: String,
    settings: Settings,
    state: State<'_, Arc<SettingsManager>>,
) -> TauriApiResult<crate::utils::EmptyData> {
    match state
        .update_workspace_settings(PathBuf::from(&workspace), &settings)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}

#[tauri::command]
pub async fn get_effective_settings(
    workspace: Option<String>,
    state: State<'_, Arc<SettingsManager>>,
) -> TauriApiResult<EffectiveSettings> {
    let workspace = workspace.map(PathBuf::from);
    match state.get_effective_settings(workspace).await {
        Ok(settings) => Ok(api_success!(settings)),
        Err(e) => Ok(ApiResponse::error(e.to_string())),
    }
}
