/*!
 * 语言设置相关的Tauri命令
 *
 * 提供前端调用的语言设置和获取接口
 */

use crate::ai::tool::storage::StorageCoordinatorState;
use crate::utils::{EmptyData, Language, LanguageManager, TauriApiResult};
use crate::{api_error, api_success};
use serde_json::Value;
use tauri::{Emitter, State};

/// 设置应用程序语言
///
/// # Arguments
/// * `language` - 语言字符串，如 "zh-CN", "en-US"
#[tauri::command]
pub async fn set_app_language<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, StorageCoordinatorState>,
    language: String,
) -> TauriApiResult<EmptyData> {
    // 验证语言格式
    let lang = Language::from_str(&language);

    // 设置后端语言管理器
    if !LanguageManager::set_language(lang) {
        return Ok(api_error!("common.system_error"));
    }

    // 保存到配置文件中的 app.language
    if let Err(_) = state
        .coordinator
        .update_config("app.language", Value::String(language.clone()))
        .await
    {
        return Ok(api_error!("config.update_failed"));
    }

    // 广播语言变更事件，供前端回显
    let _ = app.emit("language-changed", &language);

    Ok(api_success!())
}

/// 获取当前应用程序语言
#[tauri::command]
pub async fn get_app_language() -> TauriApiResult<String> {
    let lang = LanguageManager::get_language_string();
    Ok(api_success!(lang))
}
/// 获取所有支持的语言列表
#[tauri::command]
pub async fn get_supported_languages() -> TauriApiResult<Vec<LanguageInfo>> {
    let languages: Vec<LanguageInfo> = Language::all()
        .into_iter()
        .map(|lang| LanguageInfo {
            code: lang.to_string(),
            name: lang.display_name().to_string(),
        })
        .collect();

    Ok(api_success!(languages))
}

/// 语言信息结构
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LanguageInfo {
    /// 语言代码，如 "zh-CN"
    pub code: String,
    /// 语言显示名称，如 "简体中文"
    pub name: String,
}
