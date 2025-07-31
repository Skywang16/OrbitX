/*!
 * 主题服务
 *
 * 根据配置文件中的主题设置，决定当前应该使用哪个主题。
 * 支持跟随系统主题和手动选择主题两种模式。
 */

use crate::config::theme::ThemeManager;
use crate::config::types::ThemeConfig;
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
    pub fn new(theme_manager: Arc<ThemeManager>) -> Self {
        Self { theme_manager }
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
    ) -> AppResult<crate::config::types::Theme> {
        let theme_name = self.get_current_theme_name(theme_config, is_system_dark);

        match self.theme_manager.load_theme(&theme_name).await {
            Ok(theme) => Ok(theme),
            Err(e) => {
                warn!("主题加载失败: {} - {}", theme_name, e);

                // 尝试加载后备主题
                let fallback_theme = if theme_config.follow_system {
                    // 如果是跟随系统模式，尝试另一个主题
                    match is_system_dark {
                        Some(true) => &theme_config.light_theme,
                        _ => &theme_config.dark_theme,
                    }
                } else {
                    // 如果是手动模式，尝试深色主题作为后备
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

        // 检查终端主题
        if self
            .theme_manager
            .load_theme(&theme_config.terminal_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.terminal_theme.clone());
        }

        // 检查浅色主题
        if self
            .theme_manager
            .load_theme(&theme_config.light_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.light_theme.clone());
        }

        // 检查深色主题
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
        // TODO: 实现系统主题检测
        // 在 macOS 上可以使用 NSAppearance
        // 在 Windows 上可以读取注册表
        // 在 Linux 上可以检查环境变量或桌面环境设置

        #[cfg(target_os = "macos")]
        {
            // macOS 系统主题检测的占位符
            // 实际实现需要使用 Objective-C 或相关库
            Some(true) // 暂时返回深色模式
        }

        #[cfg(not(target_os = "macos"))]
        {
            // 其他系统的占位符
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::ThemeConfig;

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
        let theme_manager = Arc::new(
            crate::config::theme::ThemeManager::new(paths, options)
                .await
                .unwrap(),
        );
        let service = ThemeService::new(theme_manager);
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
        let theme_manager = Arc::new(
            crate::config::theme::ThemeManager::new(paths, options)
                .await
                .unwrap(),
        );
        let service = ThemeService::new(theme_manager);
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
        let theme_manager = Arc::new(
            crate::config::theme::ThemeManager::new(paths, options)
                .await
                .unwrap(),
        );
        let service = ThemeService::new(theme_manager);
        let mut config = create_test_theme_config();
        config.follow_system = true;

        let theme_name = service.get_current_theme_name(&config, Some(false));
        assert_eq!(theme_name, "test-light");
    }
}
