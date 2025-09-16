/*!
 * 主题服务
 *
 * 根据配置文件中的主题设置，决定当前应该使用哪个主题。
 * 支持跟随系统主题和手动选择主题两种模式。
 */

use super::manager::ThemeManager;
use super::types::{Theme, ThemeConfig};
use crate::config::paths::ConfigPaths;
use crate::config::theme::ThemeManagerOptions;
use crate::storage::cache::UnifiedCache;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::sync::Arc;
use tracing::warn;

/// 主题服务
pub struct ThemeService {
    /// 主题管理器
    theme_manager: Arc<ThemeManager>,
}

impl ThemeService {
    /// 创建新的主题服务实例
    pub async fn new(
        paths: ConfigPaths,
        options: ThemeManagerOptions,
        cache: Arc<UnifiedCache>,
    ) -> AppResult<Self> {
        let theme_manager = Arc::new(ThemeManager::new(paths, options, cache.clone()).await?);
        Ok(Self { theme_manager })
    }

    /// 获取主题管理器引用
    pub fn theme_manager(&self) -> &Arc<ThemeManager> {
        &self.theme_manager
    }

    /// 根据配置获取当前应该使用的主题名称
    ///
    /// # Arguments
    /// * `theme_config` - 主题配置
    /// * `is_system_dark` - 系统是否为深色模式（可选，用于跟随系统主题时）
    ///
    /// # Returns
    /// 返回应该使用的主题名称
    pub fn get_current_theme_name(
        &self,
        theme_config: &ThemeConfig,
        is_system_dark: Option<bool>,
    ) -> String {
        if theme_config.follow_system {
            // 跟随系统主题模式
            match is_system_dark {
                Some(true) => theme_config.dark_theme.clone(),
                Some(false) => theme_config.light_theme.clone(),
                None => theme_config.dark_theme.clone(),
            }
        } else {
            // 手动选择主题模式
            theme_config.terminal_theme.clone()
        }
    }

    /// 根据配置加载当前主题
    ///
    /// # Arguments
    /// * `theme_config` - 主题配置
    /// * `is_system_dark` - 系统是否为深色模式（可选）
    ///
    /// # Returns
    /// 返回主题数据
    pub async fn load_current_theme(
        &self,
        theme_config: &ThemeConfig,
        is_system_dark: Option<bool>,
    ) -> AppResult<Theme> {
        let theme_name = self.get_current_theme_name(theme_config, is_system_dark);

        match self.theme_manager.load_theme(&theme_name).await {
            Ok(theme) => Ok(theme),
            Err(e) => {
                warn!("主题加载失败: {} - {}", theme_name, e);

                // 尝试加载后备主题
                let fallback_theme = if theme_config.follow_system {
                    match is_system_dark {
                        Some(true) => &theme_config.light_theme,
                        _ => &theme_config.dark_theme,
                    }
                } else {
                    &theme_config.dark_theme
                };

                self.theme_manager
                    .load_theme(fallback_theme)
                    .await
                    .map_err(|fallback_err| {
                        anyhow!(
                            "主题加载失败: {} ({}), 后备主题也加载失败: {} ({})",
                            theme_name,
                            e,
                            fallback_theme,
                            fallback_err
                        )
                    })
            }
        }
    }

    /// 验证主题配置中引用的主题是否存在
    ///
    /// # Arguments
    /// * `theme_config` - 主题配置
    ///
    /// # Returns
    /// 返回验证结果和缺失的主题列表
    pub async fn validate_theme_config(
        &self,
        theme_config: &ThemeConfig,
    ) -> AppResult<Vec<String>> {
        let mut missing_themes = Vec::new();

        if self
            .theme_manager
            .load_theme(&theme_config.terminal_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.terminal_theme.clone());
        }

        if self
            .theme_manager
            .load_theme(&theme_config.light_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.light_theme.clone());
        }

        if self
            .theme_manager
            .load_theme(&theme_config.dark_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.dark_theme.clone());
        }

        if !missing_themes.is_empty() {
            warn!("发现缺失的主题: {:?}", missing_themes);
        }

        Ok(missing_themes)
    }

    /// 获取所有可用主题列表
    pub async fn list_available_themes(&self) -> AppResult<Vec<String>> {
        let themes = self.theme_manager.list_themes().await?;
        let theme_names: Vec<String> = themes.into_iter().map(|t| t.name).collect();
        Ok(theme_names)
    }

    /// 检查指定主题是否存在
    pub async fn theme_exists(&self, theme_name: &str) -> bool {
        self.theme_manager.load_theme(theme_name).await.is_ok()
    }
}

/// 系统主题检测器
pub struct SystemThemeDetector;

impl SystemThemeDetector {
    /// 检测系统是否为深色模式
    ///
    /// # Returns
    /// 返回系统主题状态，None 表示无法检测
    pub fn is_dark_mode() -> Option<bool> {
        #[cfg(target_os = "macos")]
        {
            // macOS 系统主题检测
            // 使用 osascript 命令检测系统主题
            use std::process::Command;

            let output = Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to tell appearance preferences to get dark mode"])
                .output()
                .ok()?;

            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout);
                let is_dark = result.trim().eq_ignore_ascii_case("true");
                Some(is_dark)
            } else {
                let output = Command::new("defaults")
                    .args(["read", "-g", "AppleInterfaceStyle"])
                    .output()
                    .ok()?;

                if output.status.success() {
                    let result = String::from_utf8_lossy(&output.stdout);
                    Some(result.trim().eq_ignore_ascii_case("dark"))
                } else {
                    None
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Windows 系统主题检测
            // 可以通过注册表检测，这里简化处理
            None
        }

        #[cfg(target_os = "linux")]
        {
            // Linux 系统主题检测
            if let Ok(theme) = std::env::var("GTK_THEME") {
                Some(theme.to_lowercase().contains("dark"))
            } else if let Ok(theme) = std::env::var("QT_STYLE_OVERRIDE") {
                Some(theme.to_lowercase().contains("dark"))
            } else {
                None
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            None
        }
    }

    /// 启动系统主题监听器（仅在 macOS 上）
    #[cfg(target_os = "macos")]
    pub fn start_system_theme_listener<F>(callback: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: Fn(bool) + Send + 'static,
    {
        use std::thread;
        use std::time::Duration;

        Some(thread::spawn(move || {
            let mut last_dark_mode: Option<bool> = None;

            loop {
                let current_dark_mode = Self::is_dark_mode();

                if current_dark_mode != last_dark_mode {
                    if let Some(is_dark) = current_dark_mode {
                        callback(is_dark);
                    }
                    last_dark_mode = current_dark_mode;
                }

                // 每秒检查一次主题变化
                thread::sleep(Duration::from_secs(1));
            }
        }))
    }

    /// 启动系统主题监听器（非 macOS 平台的空实现）
    #[cfg(not(target_os = "macos"))]
    pub fn start_system_theme_listener<F>(_callback: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: Fn(bool) + Send + 'static,
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::ThemeConfig;
    use crate::storage::cache::UnifiedCache;

    fn create_test_theme_config() -> ThemeConfig {
        ThemeConfig {
            auto_switch_time: "18:00".to_string(),
            terminal_theme: "test-theme".to_string(),
            light_theme: "test-light".to_string(),
            dark_theme: "test-dark".to_string(),
            follow_system: false,
        }
    }

    #[tokio::test]
    async fn test_get_current_theme_name_manual_mode() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let config = create_test_theme_config();

        let theme_name = service.get_current_theme_name(&config, Some(true));
        assert_eq!(theme_name, "test-theme");
    }

    #[tokio::test]
    async fn test_get_current_theme_name_follow_system_dark() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let mut config = create_test_theme_config();
        config.follow_system = true;

        let theme_name = service.get_current_theme_name(&config, Some(true));
        assert_eq!(theme_name, "test-dark");
    }

    #[tokio::test]
    async fn test_get_current_theme_name_follow_system_light() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let mut config = create_test_theme_config();
        config.follow_system = true;

        let theme_name = service.get_current_theme_name(&config, Some(false));
        assert_eq!(theme_name, "test-light");
    }
}
