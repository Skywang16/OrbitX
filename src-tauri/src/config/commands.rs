/*!
 * 配置系统 Tauri 命令接口
 *
 * 提供前端调用的配置管理命令，包括配置获取、更新、重载和验证等功能。
 * 实现统一的错误处理和响应格式。
 */

use crate::config::{
    defaults::create_default_config,
    paths::ConfigPaths,
    theme::{
        ThemeIndexEntry, ThemeManager, ThemeManagerOptions, ThemeValidationResult, ThemeValidator,
    },
    types::{AppConfig, Theme},
};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Runtime, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// 配置管理器状态
pub struct ConfigManagerState {
    /// 当前配置
    pub config: Mutex<AppConfig>,
    /// 配置文件路径
    pub config_path: PathBuf,
    /// 主题管理器
    pub theme_manager: Mutex<Option<ThemeManager>>,
}

impl ConfigManagerState {
    /// 创建新的配置管理器状态
    pub async fn new() -> AppResult<Self> {
        let config_path = Self::get_default_config_path()?;
        let config = Self::load_or_create_default_config(&config_path).await?;

        // 初始化主题管理器
        let theme_manager = match Self::initialize_theme_manager().await {
            Ok(manager) => Some(manager),
            Err(e) => {
                tracing::warn!("主题管理器初始化失败: {}", e);
                None
            }
        };

        Ok(Self {
            config: Mutex::new(config),
            config_path,
            theme_manager: Mutex::new(theme_manager),
        })
    }

    /// 获取默认配置文件路径
    fn get_default_config_path() -> AppResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("无法获取配置目录"))?
            .join("termx");

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).context("创建配置目录失败")?;
        }

        Ok(config_dir.join("config.toml"))
    }

    /// 加载或创建默认配置
    async fn load_or_create_default_config(config_path: &PathBuf) -> AppResult<AppConfig> {
        if config_path.exists() {
            // 尝试加载现有配置
            match tokio::fs::read_to_string(config_path).await {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(config) => {
                            tracing::debug!("配置文件解析成功");
                            Ok(config)
                        }
                        Err(e) => {
                            tracing::warn!("配置文件解析失败: {}, 使用默认配置", e);
                            let default_config = create_default_config();
                            // 尝试保存默认配置，失败也不影响返回
                            if let Err(save_err) =
                                Self::save_config_to_file(&default_config, config_path).await
                            {
                                tracing::error!("保存默认配置失败: {}", save_err);
                            }
                            Ok(default_config)
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("读取配置文件失败: {}, 使用默认配置", e);
                    let default_config = create_default_config();
                    // 尝试保存默认配置，失败也不影响返回
                    if let Err(save_err) =
                        Self::save_config_to_file(&default_config, config_path).await
                    {
                        tracing::error!("保存默认配置失败: {}", save_err);
                    }
                    Ok(default_config)
                }
            }
        } else {
            // 创建默认配置
            let default_config = create_default_config();
            Self::save_config_to_file(&default_config, config_path).await?;
            Ok(default_config)
        }
    }

    /// 保存配置到文件
    pub async fn save_config_to_file(config: &AppConfig, config_path: &PathBuf) -> AppResult<()> {
        let content = toml::to_string_pretty(config).context("序列化配置失败")?;

        tokio::fs::write(config_path, content)
            .await
            .context("写入配置文件失败")?;

        Ok(())
    }

    /// 初始化主题管理器
    async fn initialize_theme_manager() -> AppResult<ThemeManager> {
        let paths = ConfigPaths::new().context("创建配置路径失败")?;
        let options = ThemeManagerOptions::default();

        ThemeManager::new(paths, options)
            .await
            .context("创建主题管理器失败")
    }
}

/// 配置文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileInfo {
    /// 配置文件路径
    pub path: String,
    /// 文件是否存在
    pub exists: bool,
    /// 文件大小（字节）
    pub size: Option<u64>,
    /// 最后修改时间
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否可读
    pub readable: bool,
    /// 是否可写
    pub writable: bool,
}

// ============================================================================
// 配置管理命令
// ============================================================================

/// 获取当前配置
///
/// # Returns
/// 返回当前的配置结构
#[tauri::command]
pub async fn get_config(state: State<'_, ConfigManagerState>) -> Result<AppConfig, String> {
    let config = state.config.lock().await;
    let result = config.clone();

    Ok(result)
}

/// 更新配置
///
/// # Arguments
/// * `new_config` - 新的配置数据
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_config(
    new_config: AppConfig,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().await;
        *config = new_config.clone();
    }

    // 保存到文件
    ConfigManagerState::save_config_to_file(&new_config, &state.config_path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 保存配置
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn save_config(state: State<'_, ConfigManagerState>) -> Result<(), String> {
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存配置失败: {}", e);
            e.to_string()
        })?;

    Ok(())
}

/// 验证配置
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_config(state: State<'_, ConfigManagerState>) -> Result<(), String> {
    debug!("开始验证配置");

    let config = state.config.lock().await;

    // 基本验证 - 检查必要字段
    if config.version.is_empty() {
        return Err("配置版本不能为空".to_string());
    }

    if config.app.language.is_empty() {
        return Err("语言设置不能为空".to_string());
    }

    info!("配置验证通过");
    Ok(())
}

/// 重置配置为默认值
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn reset_config_to_defaults(state: State<'_, ConfigManagerState>) -> Result<(), String> {
    debug!("开始重置配置为默认值");

    let default_config = create_default_config();

    {
        let mut config = state.config.lock().await;
        *config = default_config.clone();
    }

    // 保存到文件
    ConfigManagerState::save_config_to_file(&default_config, &state.config_path)
        .await
        .map_err(|e| {
            error!("重置配置失败: {}", e);
            e.to_string()
        })?;

    info!("配置已重置为默认值");
    Ok(())
}

// ============================================================================
// 配置文件管理命令
// ============================================================================

/// 获取配置文件路径
///
/// # Returns
/// 返回配置文件的完整路径
#[tauri::command]
pub async fn get_config_file_path(state: State<'_, ConfigManagerState>) -> Result<String, String> {
    let path_str = state.config_path.to_string_lossy().to_string();

    Ok(path_str)
}

/// 获取配置文件信息
///
/// # Returns
/// 返回配置文件的详细信息
#[tauri::command]
pub async fn get_config_file_info(
    state: State<'_, ConfigManagerState>,
) -> Result<ConfigFileInfo, String> {
    let path = &state.config_path;
    let path_str = path.to_string_lossy().to_string();

    let file_info = if path.exists() {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| {
            error!("获取文件元数据失败: {}", e);
            format!("获取文件元数据失败: {}", e)
        })?;

        let modified_at = metadata.modified().ok().and_then(|time| {
            chrono::DateTime::from_timestamp(
                time.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                0,
            )
        });

        // 检查文件权限
        let readable = tokio::fs::File::open(path).await.is_ok();
        let writable = {
            // 尝试以追加模式打开文件来检查写权限
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .await
                .is_ok()
        };

        ConfigFileInfo {
            path: path_str,
            exists: true,
            size: Some(metadata.len()),
            modified_at,
            readable,
            writable,
        }
    } else {
        ConfigFileInfo {
            path: path_str,
            exists: false,
            size: None,
            modified_at: None,
            readable: false,
            writable: false,
        }
    };

    info!("配置文件信息: {:?}", file_info);
    Ok(file_info)
}

/// 打开配置文件
///
/// 使用系统默认编辑器打开配置文件
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn open_config_file<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    info!("开始打开配置文件");

    let path = &state.config_path;

    if !path.exists() {
        let error_msg = "配置文件不存在";
        error!("{}", error_msg);
        return Err(error_msg.to_string());
    }

    // 使用 tauri-plugin-opener 打开文件
    let path_str = path.to_string_lossy().to_string();

    app.opener()
        .open_path(path_str, None::<String>)
        .map_err(|e| {
            error!("打开配置文件失败: {}", e);
            format!("打开配置文件失败: {}", e)
        })?;

    info!("配置文件已打开");
    Ok(())
}

// ============================================================================
// 配置事件监听命令
// ============================================================================

/// 订阅配置事件
///
/// 前端可以通过此命令订阅配置变更事件
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn subscribe_config_events(_state: State<'_, ConfigManagerState>) -> Result<(), String> {
    debug!("订阅配置事件");

    // 简化实现 - 在实际应用中，这里可以实现事件转发到前端的逻辑
    // 由于 Tauri 的限制，我们可能需要使用其他方式来实现实时事件推送

    info!("配置事件订阅成功");
    Ok(())
}

// ============================================================================
// 主题系统命令
// ============================================================================

/// 获取所有可用主题列表
///
/// # Returns
/// 返回主题列表
#[tauri::command]
pub async fn get_theme_list(
    state: State<'_, ConfigManagerState>,
) -> Result<Vec<ThemeIndexEntry>, String> {
    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.list_themes().await {
            Ok(themes) => Ok(themes),
            Err(e) => {
                error!("获取主题列表失败: {}", e);
                Err(format!("获取主题列表失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

/// 加载指定主题
///
/// # Arguments
/// * `theme_name` - 主题名称
///
/// # Returns
/// 返回主题数据
#[tauri::command]
pub async fn load_theme(
    theme_name: String,
    state: State<'_, ConfigManagerState>,
) -> Result<Theme, String> {
    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.load_theme(&theme_name).await {
            Ok(theme) => Ok(theme),
            Err(e) => {
                error!("加载主题失败: {}", e);
                Err(format!("加载主题失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

/// 切换当前主题
///
/// # Arguments
/// * `theme_name` - 新主题名称
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn switch_theme(
    theme_name: String,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始切换主题: {}", theme_name);

    // 首先验证主题是否存在，如果不存在则尝试自动修复
    {
        let theme_manager_guard = state.theme_manager.lock().await;
        if let Some(ref theme_manager) = *theme_manager_guard {
            if let Err(e) = theme_manager.load_theme(&theme_name).await {
                warn!("主题不存在或加载失败: {}，尝试自动修复", e);

                // 如果是内置主题（light 或 dark），尝试重新创建
                if theme_name == "light" || theme_name == "dark" {
                    info!("尝试重新创建内置主题: {}", theme_name);
                    if let Err(create_err) = theme_manager.create_builtin_themes().await {
                        error!("重新创建内置主题失败: {}", create_err);
                        return Err(format!("主题不存在且自动修复失败: {}", create_err));
                    }

                    // 重新尝试加载主题
                    if let Err(retry_err) = theme_manager.load_theme(&theme_name).await {
                        error!("重新创建后仍无法加载主题: {}", retry_err);
                        return Err(format!("主题自动修复失败: {}", retry_err));
                    }

                    info!("内置主题自动修复成功: {}", theme_name);
                } else {
                    error!("非内置主题不存在: {}", theme_name);
                    return Err(format!(
                        "主题不存在: {}。请检查主题文件是否正确安装。",
                        theme_name
                    ));
                }
            }
        } else {
            let error_msg = "主题管理器未初始化";
            error!("{}", error_msg);
            return Err(error_msg.to_string());
        }
    }

    // 更新配置中的当前主题
    {
        let mut config = state.config.lock().await;
        config.appearance.theme_config.terminal_theme = theme_name.clone();
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存配置失败: {}", e);
            e.to_string()
        })?;

    info!("主题切换成功: {}", theme_name);
    Ok(())
}

/// 验证主题文件
///
/// # Arguments
/// * `theme_name` - 主题名称
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_theme(
    theme_name: String,
    state: State<'_, ConfigManagerState>,
) -> Result<ThemeValidationResult, String> {
    debug!("开始验证主题: {}", theme_name);

    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.load_theme(&theme_name).await {
            Ok(theme) => {
                let validation_result = ThemeValidator::validate_theme(&theme);
                info!("主题验证完成: {}", theme_name);
                Ok(validation_result)
            }
            Err(e) => {
                error!("验证主题失败: {}", e);
                Err(format!("验证主题失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

/// 刷新主题索引
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn refresh_theme_index(state: State<'_, ConfigManagerState>) -> Result<(), String> {
    debug!("开始刷新主题索引");

    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.refresh_index().await {
            Ok(_) => {
                info!("主题索引刷新成功");
                Ok(())
            }
            Err(e) => {
                error!("刷新主题索引失败: {}", e);
                Err(format!("刷新主题索引失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

/// 创建内置主题文件
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn create_builtin_themes(state: State<'_, ConfigManagerState>) -> Result<(), String> {
    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.create_builtin_themes().await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("创建内置主题文件失败: {}", e);
                Err(format!("创建内置主题文件失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

/// 获取主题索引
///
/// # Returns
/// 返回主题索引信息
#[tauri::command]
pub async fn get_theme_index(
    state: State<'_, ConfigManagerState>,
) -> Result<crate::config::theme::ThemeIndex, String> {
    debug!("开始获取主题索引");

    let theme_manager_guard = state.theme_manager.lock().await;

    if let Some(ref theme_manager) = *theme_manager_guard {
        match theme_manager.get_theme_index().await {
            Ok(index) => {
                info!("成功获取主题索引");
                Ok(index)
            }
            Err(e) => {
                error!("获取主题索引失败: {}", e);
                Err(format!("获取主题索引失败: {}", e))
            }
        }
    } else {
        let error_msg = "主题管理器未初始化";
        error!("{}", error_msg);
        Err(error_msg.to_string())
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::test;

    /// 创建测试用的配置管理器状态
    async fn create_test_state() -> (ConfigManagerState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let config = create_default_config();

        let state = ConfigManagerState {
            config: Mutex::new(config),
            config_path,
            theme_manager: Mutex::new(None), // 测试中不初始化主题管理器
        };

        (state, temp_dir)
    }

    #[test]
    async fn test_get_config() {
        let (state, _temp_dir) = create_test_state().await;

        let config = state.config.lock().await;
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");
    }

    #[test]
    async fn test_config_file_path() {
        let (state, _temp_dir) = create_test_state().await;

        let path_str = state.config_path.to_string_lossy().to_string();
        assert!(!path_str.is_empty());
        assert!(path_str.ends_with("config.toml"));
    }

    #[test]
    async fn test_validate_config() {
        let (state, _temp_dir) = create_test_state().await;

        let config = state.config.lock().await;
        assert!(!config.version.is_empty());
        assert!(!config.app.language.is_empty());
    }

    #[test]
    async fn test_save_and_load_config() {
        let (state, _temp_dir) = create_test_state().await;

        let config = state.config.lock().await.clone();

        // 保存配置
        let save_result =
            ConfigManagerState::save_config_to_file(&config, &state.config_path).await;
        assert!(save_result.is_ok());

        // 验证文件存在
        assert!(state.config_path.exists());

        // 重新加载配置
        let loaded_config =
            ConfigManagerState::load_or_create_default_config(&state.config_path).await;
        assert!(loaded_config.is_ok());

        let loaded_config = loaded_config.unwrap();
        assert_eq!(loaded_config.version, config.version);
        assert_eq!(loaded_config.app.language, config.app.language);
    }

    #[test]
    async fn test_reset_config_to_defaults() {
        let (state, _temp_dir) = create_test_state().await;

        // 修改配置
        {
            let mut config = state.config.lock().await;
            config.app.language = "en-US".to_string();
        }

        // 重置为默认值
        let default_config = create_default_config();
        {
            let mut config = state.config.lock().await;
            *config = default_config;
        }

        // 验证配置已重置
        let config = state.config.lock().await;
        assert_eq!(config.app.language, "zh-CN");
    }
}
