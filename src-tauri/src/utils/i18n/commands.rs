use crate::ai::tool::storage::StorageCoordinatorState;
use crate::utils::{EmptyData, Language, LanguageManager, TauriApiResult};
use crate::{api_error, api_success};
use serde_json::Value;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn language_set_app_language<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, StorageCoordinatorState>,
    language: String,
) -> TauriApiResult<EmptyData> {
    let lang = Language::from_str(&language);

    if !LanguageManager::set_language(lang) {
        return Ok(api_error!("common.system_error"));
    }

    if let Err(_) = state
        .coordinator
        .config_update("app.language", Value::String(language.clone()))
        .await
    {
        return Ok(api_error!("config.update_failed"));
    }

    let _ = app.emit("language-changed", &language);

    Ok(api_success!())
}

#[tauri::command]
pub async fn language_get_app_language() -> TauriApiResult<String> {
    let lang = LanguageManager::get_language_string();
    Ok(api_success!(lang))
}

#[tauri::command]
pub async fn language_get_supported_languages() -> TauriApiResult<Vec<LanguageInfo>> {
    let languages: Vec<LanguageInfo> = Language::all()
        .into_iter()
        .map(|lang| LanguageInfo {
            code: lang.to_string(),
            name: lang.display_name().to_string(),
        })
        .collect();

    Ok(api_success!(languages))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
}
