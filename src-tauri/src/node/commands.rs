use super::detector::{detect_version_manager, get_node_versions};
use super::types::{NodeVersionInfo, NodeVersionManager};
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use std::path::Path;
use std::str::FromStr;

#[tauri::command]
pub async fn node_check_project(path: String) -> TauriApiResult<bool> {
    let package_json_path = Path::new(&path).join("package.json");
    Ok(api_success!(package_json_path.exists()))
}

#[tauri::command]
pub async fn node_get_version_manager() -> TauriApiResult<String> {
    let manager = detect_version_manager();
    Ok(api_success!(manager.as_str().to_string()))
}

#[tauri::command]
pub async fn node_list_versions() -> TauriApiResult<Vec<NodeVersionInfo>> {
    let manager = detect_version_manager();
    match get_node_versions(&manager) {
        Ok(versions) => {
            let current_version = super::detector::get_current_version(None).ok().flatten();
            let version_infos = versions
                .into_iter()
                .map(|v| {
                    let is_current = current_version.as_ref().is_some_and(|current| {
                        v.trim_start_matches('v') == current.trim_start_matches('v')
                    });
                    NodeVersionInfo {
                        is_current,
                        version: v,
                    }
                })
                .collect();
            Ok(api_success!(version_infos))
        }
        Err(e) => {
            tracing::error!("Failed to list node versions: {}", e);
            Ok(api_error!("node.list_versions_failed"))
        }
    }
}

#[tauri::command]
pub async fn node_get_switch_command(manager: String, version: String) -> TauriApiResult<String> {
    let mgr = NodeVersionManager::from_str(&manager).unwrap_or(NodeVersionManager::Unknown);
    let version_cleaned = version.trim().trim_start_matches('v');

    let command = match mgr {
        NodeVersionManager::Nvm => format!("nvm use {version_cleaned}\n"),
        NodeVersionManager::Fnm => format!("fnm use {version_cleaned}\n"),
        NodeVersionManager::Volta => format!("volta install node@{version_cleaned}\n"),
        NodeVersionManager::N => format!("n {version_cleaned}\n"),
        NodeVersionManager::Asdf => format!("asdf global nodejs {version_cleaned}\n"),
        NodeVersionManager::Unknown => {
            return Ok(api_error!("node.unknown_version_manager"));
        }
    };

    Ok(api_success!(command))
}
