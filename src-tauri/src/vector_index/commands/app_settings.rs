/*!
 * 向量索引应用级设置管理命令
 *
 * 管理向量索引功能的全局开关和工作目录配置
 * 数据存储在mp层(MessagePack)中
 */

use crate::ai::tool::storage::StorageCoordinatorState;
use crate::utils::error::ToTauriResult;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorIndexAppSettings {
    /// 向量索引功能是否启用
    pub enabled: bool,
    /// 支持向量索引的工作目录列表（最多3个）
    pub workspaces: Vec<String>,
}

impl Default for VectorIndexAppSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            workspaces: Vec::new(),
        }
    }
}

/// 获取向量索引应用设置
#[tauri::command]
pub async fn get_vector_index_app_settings(
    state: State<'_, StorageCoordinatorState>,
) -> Result<VectorIndexAppSettings, String> {
    debug!("获取向量索引应用设置");

    let session_state = state
        .coordinator
        .load_session_state()
        .await
        .to_tauri()?
        .unwrap_or_default();

    let settings = VectorIndexAppSettings {
        enabled: session_state.ai.vector_index_enabled,
        workspaces: session_state.ai.vector_index_workspaces.clone(),
    };

    debug!("向量索引应用设置获取成功: {:?}", settings);
    Ok(settings)
}

/// 保存向量索引应用设置
#[tauri::command]
pub async fn save_vector_index_app_settings(
    settings: VectorIndexAppSettings,
    state: State<'_, StorageCoordinatorState>,
) -> Result<String, String> {
    info!("保存向量索引应用设置: {:?}", settings);

    // 验证工作目录数量
    if settings.workspaces.len() > 3 {
        return Err("最多只能支持3个工作目录".to_string());
    }

    let mut session_state = state
        .coordinator
        .load_session_state()
        .await
        .to_tauri()?
        .unwrap_or_default();

    // 更新AI状态中的向量索引设置
    session_state.ai.vector_index_enabled = settings.enabled;
    session_state.ai.vector_index_workspaces = settings.workspaces.clone();

    // 更新时间戳
    session_state.timestamp = chrono::Utc::now();

    // 保存到存储
    state
        .coordinator
        .save_session_state(&session_state)
        .await
        .to_tauri()?;

    info!("向量索引应用设置保存成功");
    Ok("向量索引应用设置保存成功".to_string())
}

/// 检查指定目录是否启用了向量索引
#[tauri::command]
pub async fn is_directory_vector_indexed(
    directory: String,
    state: State<'_, StorageCoordinatorState>,
) -> Result<bool, String> {
    debug!("检查目录是否启用向量索引: {}", directory);

    let session_state = state
        .coordinator
        .load_session_state()
        .await
        .to_tauri()?
        .unwrap_or_default();

    // 如果功能未启用，直接返回false
    if !session_state.ai.vector_index_enabled {
        return Ok(false);
    }

    // 检查目录是否在配置的工作目录列表中
    let is_indexed = session_state
        .ai
        .vector_index_workspaces
        .iter()
        .any(|workspace| {
            // 支持精确匹配或子目录匹配
            directory == *workspace || directory.starts_with(&format!("{}/", workspace))
        });

    debug!("目录 {} 是否启用向量索引: {}", directory, is_indexed);
    Ok(is_indexed)
}

/// 添加工作目录到向量索引配置
#[tauri::command]
pub async fn add_vector_index_workspace(
    workspace_path: String,
    state: State<'_, StorageCoordinatorState>,
) -> Result<String, String> {
    info!("添加向量索引工作目录: {}", workspace_path);

    let mut session_state = state
        .coordinator
        .load_session_state()
        .await
        .to_tauri()?
        .unwrap_or_default();

    // 检查是否已存在
    if session_state
        .ai
        .vector_index_workspaces
        .contains(&workspace_path)
    {
        return Err("工作目录已存在于配置中".to_string());
    }

    // 检查数量限制
    if session_state.ai.vector_index_workspaces.len() >= 3 {
        return Err("最多只能配置3个工作目录".to_string());
    }

    // 添加工作目录
    session_state
        .ai
        .vector_index_workspaces
        .push(workspace_path.clone());
    session_state.timestamp = chrono::Utc::now();

    // 保存到存储
    state
        .coordinator
        .save_session_state(&session_state)
        .await
        .to_tauri()?;

    info!("向量索引工作目录添加成功");
    Ok("向量索引工作目录添加成功".to_string())
}

/// 移除工作目录从向量索引配置
#[tauri::command]
pub async fn remove_vector_index_workspace(
    workspace_path: String,
    state: State<'_, StorageCoordinatorState>,
) -> Result<String, String> {
    info!("移除向量索引工作目录: {}", workspace_path);

    let mut session_state = state
        .coordinator
        .load_session_state()
        .await
        .to_tauri()?
        .unwrap_or_default();

    // 查找并移除工作目录
    let initial_len = session_state.ai.vector_index_workspaces.len();
    session_state
        .ai
        .vector_index_workspaces
        .retain(|w| w != &workspace_path);

    if session_state.ai.vector_index_workspaces.len() == initial_len {
        return Err("工作目录不存在于配置中".to_string());
    }

    session_state.timestamp = chrono::Utc::now();

    // 保存到存储
    state
        .coordinator
        .save_session_state(&session_state)
        .await
        .to_tauri()?;

    info!("向量索引工作目录移除成功");
    Ok("向量索引工作目录移除成功".to_string())
}
