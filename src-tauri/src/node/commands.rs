use super::detector::{detect_version_manager, get_node_versions};
use super::types::{NodeVersionInfo, NodeVersionManager};
use std::path::Path;

// 检查是否为 Node.js 项目（是否包含 package.json）
#[tauri::command]
pub async fn node_check_project(path: String) -> Result<bool, String> {
    let package_json_path = Path::new(&path).join("package.json");
    Ok(package_json_path.exists())
}

// 获取当前系统使用的 Node 版本管理器
#[tauri::command]
pub async fn node_get_version_manager() -> Result<String, String> {
    let manager = detect_version_manager();
    Ok(manager.as_str().to_string())
}

// 获取所有已安装的 Node 版本列表，并标记当前版本
#[tauri::command]
pub async fn node_list_versions() -> Result<Vec<NodeVersionInfo>, String> {
    let manager = detect_version_manager();
    let versions = get_node_versions(&manager)?;
    // 获取当前版本用于标记
    let current_version = super::detector::get_current_version(None).ok().flatten();

    let version_infos = versions
        .into_iter()
        .map(|v| {
            // 规范化后比较，移除 'v' 前缀
            let is_current = current_version.as_ref().map_or(false, |current| {
                v.trim_start_matches('v') == current.trim_start_matches('v')
            });
            NodeVersionInfo {
                is_current,
                version: v,
            }
        })
        .collect();

    Ok(version_infos)
}

// 根据版本管理器和版本号生成切换命令
#[tauri::command]
pub async fn node_get_switch_command(manager: String, version: String) -> Result<String, String> {
    let mgr = NodeVersionManager::from_str(&manager);
    let version_cleaned = version.trim().trim_start_matches('v');

    let command = match mgr {
        NodeVersionManager::Nvm => format!("nvm use {}\n", version_cleaned),
        NodeVersionManager::Fnm => format!("fnm use {}\n", version_cleaned),
        NodeVersionManager::Volta => format!("volta install node@{}\n", version_cleaned),
        NodeVersionManager::N => format!("n {}\n", version_cleaned),
        NodeVersionManager::Asdf => format!("asdf global nodejs {}\n", version_cleaned),
        NodeVersionManager::Unknown => {
            return Err("Unknown version manager".to_string());
        }
    };

    Ok(command)
}
