use crate::config::TomlConfigManager;
use crate::utils::{EmptyData, Language, LanguageManager, TauriApiResult};
use crate::{api_error, api_success};
use serde_json::Value;
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn language_set_app_language<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: State<'_, Arc<TomlConfigManager>>,
    language: String,
) -> TauriApiResult<EmptyData> {
    let lang = Language::from_str(&language);

    if !LanguageManager::set_language(lang) {
        return Ok(api_error!("common.system_error"));
    }

    if let Err(_) = config
        .inner()
        .update_section("app.language", Value::String(language.clone()))
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
