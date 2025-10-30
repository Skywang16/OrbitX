use super::types::{AnsiColors, SyntaxHighlight, Theme, ThemeType, UIColors};
use crate::config::error::{ThemeConfigError, ThemeConfigResult};
use crate::config::paths::ConfigPaths;
use crate::storage::cache::UnifiedCache;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Instant, SystemTime},
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileWrapper {
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeIndexEntry {
    pub name: String,

    pub file: String,

    #[serde(rename = "type")]
    pub theme_type: String,

    pub builtin: bool,

    pub file_size: Option<u64>,

    /// 最后修改时间
    pub last_modified: Option<SystemTime>,
}

/// 主题索引文件结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeIndex {
    /// 索引版本
    pub version: String,

    /// 最后更新时间
    pub last_updated: SystemTime,

    /// 主题总数
    pub total_themes: usize,

    /// 内置主题列表
    pub builtin_themes: Vec<ThemeIndexEntry>,

    /// 用户自定义主题列表
    pub custom_themes: Vec<ThemeIndexEntry>,

    /// 默认主题配置
    pub defaults: ThemeDefaults,
}

/// 主题默认配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDefaults {
    /// 默认浅色主题
    pub light_theme: String,

    /// 默认深色主题
    pub dark_theme: String,

    /// 默认主题
    pub default_theme: String,
}

/// 主题管理器选项
#[derive(Debug, Clone)]
pub struct ThemeManagerOptions {
    /// 自动刷新索引
    pub auto_refresh_index: bool,

    /// 索引刷新间隔（秒）
    pub index_refresh_interval: u64,
}

impl Default for ThemeManagerOptions {
    fn default() -> Self {
        Self {
            auto_refresh_index: true,
            index_refresh_interval: 300, // 5分钟
        }
    }
}

// 主题管理器核心结构

/// 主题管理器
///
/// 负责主题系统的核心功能，包括主题加载、验证、索引管理和缓存。
pub struct ThemeManager {
    /// 配置路径管理器
    paths: ConfigPaths,

    /// 主题索引
    index: Arc<RwLock<Option<ThemeIndex>>>,

    /// 统一缓存
    cache: Arc<UnifiedCache>,

    /// 管理器选项
    #[allow(dead_code)]
    options: ThemeManagerOptions,

    /// 最后索引刷新时间
    last_index_refresh: Arc<Mutex<Option<Instant>>>,
}

impl ThemeManager {
    /// 创建新的主题管理器实例
    ///
    /// # Arguments
    /// * `paths` - 配置路径管理器
    /// * `options` - 主题管理器选项
    ///
    /// # Returns
    /// 返回主题管理器实例
    pub async fn new(
        paths: ConfigPaths,
        options: ThemeManagerOptions,
        cache: Arc<UnifiedCache>,
    ) -> ThemeConfigResult<Self> {
        let manager = Self {
            paths,
            index: Arc::new(RwLock::new(None)),
            cache,
            options,
            last_index_refresh: Arc::new(Mutex::new(None)),
        };

        manager.ensure_theme_directories().await?;

        manager.create_builtin_themes().await?;

        // 直接刷新索引而不是加载现有索引（确保索引与实际文件同步）
        manager.refresh_index().await?;

        info!("Theme manager initialized successfully");
        Ok(manager)
    }

    /// 确保主题目录存在
    async fn ensure_theme_directories(&self) -> ThemeConfigResult<()> {
        let themes_dir = self.paths.themes_dir();

        if !themes_dir.exists() {
            fs::create_dir_all(themes_dir).map_err(|e| ThemeConfigError::Io(e))?;
            info!("创建主题目录: {}", themes_dir.display());
        }

        Ok(())
    }

    /// 加载主题索引
    async fn load_theme_index(&self) -> ThemeConfigResult<()> {
        let index_path = self.paths.themes_dir().join("index.toml");

        let theme_index = if index_path.exists() {
            self.load_index_from_file(&index_path).await?
        } else {
            self.create_default_index().await?
        };

        // 更新索引
        {
            let mut index = self.index.write().map_err(ThemeConfigError::from_poison)?;
            *index = Some(theme_index);
        }

        // 更新最后刷新时间
        {
            let mut last_refresh = self.last_index_refresh.lock().await;
            *last_refresh = Some(Instant::now());
        }

        Ok(())
    }

    /// 从文件加载索引
    async fn load_index_from_file(&self, index_path: &Path) -> ThemeConfigResult<ThemeIndex> {
        let content = tokio::fs::read_to_string(index_path).await.map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read theme index file {}: {}",
                index_path.display(),
                err
            ))
        })?;

        let index: ThemeIndex = toml::from_str(&content).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to parse theme index file {}: {}",
                index_path.display(),
                err
            ))
        })?;

        debug!(
            "Theme index loaded successfully with {} themes",
            index.total_themes
        );
        Ok(index)
    }

    /// 创建默认索引
    async fn create_default_index(&self) -> ThemeConfigResult<ThemeIndex> {
        let index = ThemeIndex {
            version: "1.0.0".to_string(),
            last_updated: SystemTime::now(),
            total_themes: 0,
            builtin_themes: Vec::new(),
            custom_themes: Vec::new(),
            defaults: ThemeDefaults {
                light_theme: "light".to_string(),
                dark_theme: "dark".to_string(),
                default_theme: "dark".to_string(),
            },
        };

        // 保存默认索引到文件
        self.save_index_to_file(&index).await?;

        Ok(index)
    }

    /// 保存索引到文件
    async fn save_index_to_file(&self, index: &ThemeIndex) -> ThemeConfigResult<()> {
        let index_path = self.paths.themes_dir().join("index.toml");

        let content = toml::to_string_pretty(index).map_err(|err| {
            ThemeConfigError::Internal(format!("Failed to serialize theme index: {}", err))
        })?;

        tokio::fs::write(&index_path, content)
            .await
            .map_err(|err| {
                ThemeConfigError::Internal(format!(
                    "Failed to write theme index file {}: {}",
                    index_path.display(),
                    err
                ))
            })?;

        debug!("Theme index saved to: {}", index_path.display());
        Ok(())
    }

    /// 加载主题
    ///
    /// # Arguments
    /// * `theme_name` - 主题名称
    ///
    /// # Returns
    /// 返回主题数据
    pub async fn load_theme(&self, theme_name: &str) -> ThemeConfigResult<Theme> {
        let cache_key = format!("theme:{}", theme_name);

        // 尝试从缓存获取
        if let Some(cached_value) = self.cache.get(&cache_key).await {
            if let Ok(theme) = serde_json::from_value(cached_value) {
                return Ok(theme);
            }
        }

        // 缓存未命中，从文件加载
        let theme = self.load_theme_from_file(theme_name).await?;

        // 存入缓存
        if let Ok(theme_value) = serde_json::to_value(&theme) {
            let _ = self.cache.set(&cache_key, theme_value).await;
        }

        Ok(theme)
    }

    /// 从文件加载主题
    async fn load_theme_from_file(&self, theme_name: &str) -> ThemeConfigResult<Theme> {
        let theme_path = self.get_theme_file_path(theme_name).await?;

        let content = tokio::fs::read_to_string(&theme_path)
            .await
            .map_err(|err| {
                ThemeConfigError::Internal(format!(
                    "Failed to read theme file {}: {}",
                    theme_path.display(),
                    err
                ))
            })?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to parse theme file {}: {}\n{}",
                theme_path.display(),
                err,
                content
            ))
        })?;

        let theme = theme_wrapper.theme;

        // 验证主题
        let validation_result = ThemeValidator::validate_theme(&theme);
        if !validation_result.is_valid {
            return Err(ThemeConfigError::Validation {
                reason: format!("Theme validation failed: {:?}", validation_result.errors),
            });
        }

        if !validation_result.warnings.is_empty() {
            warn!(
                "Theme {} has warnings: {:?}",
                theme_name, validation_result.warnings
            );
        }

        Ok(theme)
    }

    /// 获取主题文件路径
    async fn get_theme_file_path(&self, theme_name: &str) -> ThemeConfigResult<PathBuf> {
        let themes_dir = self.paths.themes_dir();

        // 直接在 themes 目录下查找主题文件
        let theme_path = themes_dir.join(format!("{}.toml", theme_name));
        if theme_path.exists() {
            return Ok(theme_path);
        }

        Err(ThemeConfigError::NotFound {
            name: theme_name.to_string(),
        })
    }

    /// 获取所有可用主题列表
    pub async fn list_themes(&self) -> ThemeConfigResult<Vec<ThemeIndexEntry>> {
        // 首先尝试获取现有索引
        let themes = {
            let index = self.index.read().map_err(ThemeConfigError::from_poison)?;

            if let Some(ref theme_index) = *index {
                let mut themes = Vec::new();
                themes.extend(theme_index.builtin_themes.clone());
                themes.extend(theme_index.custom_themes.clone());

                debug!(
                    "Theme list fetched: {} builtin themes, {} custom themes",
                    theme_index.builtin_themes.len(),
                    theme_index.custom_themes.len()
                );

                Some(themes)
            } else {
                None
            }
        };

        if themes.is_none() {
            warn!("Theme index not initialized, attempting reload");
            self.load_theme_index().await?;

            let index = self.index.read().map_err(ThemeConfigError::from_poison)?;

            if let Some(ref theme_index) = *index {
                let mut themes = Vec::new();
                themes.extend(theme_index.builtin_themes.clone());
                themes.extend(theme_index.custom_themes.clone());
                return Ok(themes);
            } else {
                return Err(ThemeConfigError::Internal(
                    "Theme index still not initialized after reload".to_string(),
                ));
            }
        }

        let themes = themes.unwrap();

        if themes.is_empty() {
            warn!("Theme list empty, attempting to refresh index");
            self.refresh_index().await?;

            let index = self.index.read().map_err(ThemeConfigError::from_poison)?;

            if let Some(ref theme_index) = *index {
                let mut refreshed_themes = Vec::new();
                refreshed_themes.extend(theme_index.builtin_themes.clone());
                refreshed_themes.extend(theme_index.custom_themes.clone());
                info!("Fetched {} themes after refresh", refreshed_themes.len());
                return Ok(refreshed_themes);
            }
        }

        Ok(themes)
    }

    /// 获取主题索引
    pub async fn get_theme_index(&self) -> ThemeConfigResult<ThemeIndex> {
        let index = self.index.read().map_err(ThemeConfigError::from_poison)?;

        if let Some(ref theme_index) = *index {
            Ok(theme_index.clone())
        } else {
            Err(ThemeConfigError::Internal(
                "Theme index not initialized".to_string(),
            ))
        }
    }

    /// 刷新主题索引
    pub async fn refresh_index(&self) -> ThemeConfigResult<()> {
        info!("Starting theme index refresh");

        // 只扫描主题目录，不区分内置和自定义主题
        let themes_dir = self.paths.themes_dir();
        let all_themes = self.scan_theme_directory(themes_dir, false).await?;

        // 根据主题索引文件中的信息区分内置和自定义主题
        let (builtin_themes, custom_themes) = self.categorize_themes(all_themes).await;

        let total_themes = custom_themes.len() + builtin_themes.len();
        let new_index = ThemeIndex {
            version: "1.0.0".to_string(),
            last_updated: SystemTime::now(),
            total_themes,
            builtin_themes,
            custom_themes,
            defaults: ThemeDefaults {
                light_theme: "light".to_string(),
                dark_theme: "dark".to_string(),
                default_theme: "dark".to_string(),
            },
        };

        // 清空旧的主题缓存
        let theme_keys_to_remove: Vec<String> = self
            .cache
            .keys()
            .await
            .into_iter()
            .filter(|k| k.starts_with("theme:"))
            .collect();
        for key in theme_keys_to_remove {
            self.cache.remove(&key).await;
        }

        // 保存索引到文件
        self.save_index_to_file(&new_index).await?;

        // 更新内存中的索引
        {
            let mut index = self.index.write().map_err(ThemeConfigError::from_poison)?;
            *index = Some(new_index);
        }

        // 更新最后刷新时间
        {
            let mut last_refresh = self.last_index_refresh.lock().await;
            *last_refresh = Some(Instant::now());
        }

        Ok(())
    }

    /// 扫描主题目录
    async fn scan_theme_directory(
        &self,
        dir: &Path,
        is_builtin: bool,
    ) -> ThemeConfigResult<Vec<ThemeIndexEntry>> {
        let mut themes = Vec::new();

        if !dir.exists() {
            return Ok(themes);
        }

        let mut entries = tokio::fs::read_dir(dir).await.map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read directory {}: {}",
                dir.display(),
                err
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read directory entry in {}: {}",
                dir.display(),
                err
            ))
        })? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // 跳过索引文件
                    if file_name == "index" {
                        continue;
                    }

                    match self
                        .create_theme_index_entry(&path, file_name, is_builtin)
                        .await
                    {
                        Ok(theme_entry) => themes.push(theme_entry),
                        Err(e) => {
                            warn!("Failed to create theme index entry {}: {}", file_name, e);
                        }
                    }
                }
            }
        }

        Ok(themes)
    }

    /// 创建主题索引条目
    async fn create_theme_index_entry(
        &self,
        path: &Path,
        file_name: &str,
        is_builtin: bool,
    ) -> ThemeConfigResult<ThemeIndexEntry> {
        let metadata = fs::metadata(path).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to get file metadata {}: {}",
                path.display(),
                err
            ))
        })?;

        let file_size = metadata.len();
        let last_modified = metadata.modified().ok();

        // 尝试读取主题文件以获取元数据
        let content = tokio::fs::read_to_string(path).await.map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read theme file {}: {}",
                path.display(),
                err
            ))
        })?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to parse theme file {}: {}",
                path.display(),
                err
            ))
        })?;

        let theme = theme_wrapper.theme;

        Ok(ThemeIndexEntry {
            name: theme.name,
            file: format!("{}.toml", file_name),
            theme_type: theme.theme_type.to_string(),
            builtin: is_builtin,
            file_size: Some(file_size),
            last_modified,
        })
    }

    /// 创建内置主题文件
    ///
    /// 从打包的资源目录复制主题文件到用户配置目录
    pub async fn create_builtin_themes(&self) -> ThemeConfigResult<()> {
        let themes_dir = self.paths.themes_dir();

        if !themes_dir.exists() {
            fs::create_dir_all(themes_dir).map_err(|err| {
                ThemeConfigError::Internal(format!(
                    "Failed to create theme directory {}: {}",
                    themes_dir.display(),
                    err
                ))
            })?;
        }

        // 检查已有的主题文件数量
        if let Ok(entries) = fs::read_dir(themes_dir) {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
                .count();

            if count > 0 {
                // 已有主题文件，刷新索引后返回
                let _ = self.refresh_index().await;
                return Ok(());
            }
        }

        // 没有主题文件时创建默认主题作为后备
        self.ensure_default_themes_exist(themes_dir).await?;
        let _ = self.refresh_index().await;

        Ok(())
    }

    /// 扫描主题目录并加载现有主题文件
    #[allow(dead_code)]
    async fn scan_and_load_existing_themes(&self, themes_dir: &Path) -> ThemeConfigResult<()> {
        if !themes_dir.exists() {
            self.create_default_theme_files(themes_dir).await?;
            return Ok(());
        }

        // 扫描现有的 TOML 主题文件
        let entries = fs::read_dir(themes_dir).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read theme directory {}: {}",
                themes_dir.display(),
                err
            ))
        })?;

        let mut theme_count = 0;
        for entry in entries {
            let entry = entry.map_err(|err| {
                ThemeConfigError::Internal(format!("Failed to read directory entry: {}", err))
            })?;
            let path = entry.path();

            // 只处理 .toml 文件
            if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // 跳过索引文件
                    if file_name == "index" {
                        continue;
                    }

                    // 验证主题文件格式
                    match self.validate_theme_file(&path).await {
                        Ok(_) => {
                            theme_count += 1;
                        }
                        Err(e) => {
                            warn!("Invalid theme file format {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        if theme_count == 0 {
            self.create_default_theme_files(themes_dir).await?;
        }

        Ok(())
    }

    /// 验证主题文件格式
    #[allow(dead_code)]
    async fn validate_theme_file(&self, path: &Path) -> ThemeConfigResult<()> {
        let content = tokio::fs::read_to_string(path).await.map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to read theme file {}: {}",
                path.display(),
                err
            ))
        })?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content).map_err(|err| {
            ThemeConfigError::Internal(format!(
                "Failed to parse theme file {}: {}",
                path.display(),
                err
            ))
        })?;

        let theme = theme_wrapper.theme;

        // 验证主题
        let validation_result = ThemeValidator::validate_theme(&theme);
        if !validation_result.is_valid {
            return Err(ThemeConfigError::Validation {
                reason: format!("Theme validation failed: {:?}", validation_result.errors),
            });
        }

        Ok(())
    }

    /// 创建默认主题文件
    async fn create_default_theme_files(&self, themes_dir: &Path) -> ThemeConfigResult<()> {
        info!("Creating default theme files");

        let dark_theme_content = r##"[theme]
name = "dark"
theme_type = "dark"



[theme.ansi]
black = "#000000"
red = "#cd3131"
green = "#0dbc79"
yellow = "#e5e510"
blue = "#2472c8"
magenta = "#bc3fbc"
cyan = "#11a8cd"
white = "#e5e5e5"

[theme.bright]
black = "#666666"
red = "#f14c4c"
green = "#23d18b"
yellow = "#f5f543"
blue = "#3b8eea"
magenta = "#d670d6"
cyan = "#29b8db"
white = "#ffffff"

[theme.syntax]
comment = "#6a9955"
string = "#ce9178"
number = "#b5cea8"
keyword = "#569cd6"
function = "#dcdcaa"
variable = "#9cdcfe"
type_name = "#4ec9b0"
operator = "#d4d4d4"

[theme.ui]
# 背景色层次
bg_100 = "#1a1a1a"
bg_200 = "#1e1e1e"
bg_300 = "#252526"
bg_400 = "#2d2d30"
bg_500 = "#3c3c3c"
bg_600 = "#4d4d4d"
bg_700 = "#5a5a5a"

# 边框层次
border_200 = "rgba(255, 255, 255, 0.08)"
border_300 = "rgba(255, 255, 255, 0.12)"
border_400 = "rgba(255, 255, 255, 0.16)"

# 文本层次
text_100 = "#ffffff"
text_200 = "#e0e0e0"
text_300 = "#cccccc"
text_400 = "#999999"
text_500 = "#666666"

# 状态颜色
primary = "#007acc"
primary_hover = "#005a9e"
primary_alpha = "rgba(0, 122, 204, 0.1)"
success = "#0dbc79"
warning = "#ffcc02"
error = "#f44747"
info = "#75beff"

# 交互状态
hover = "#2a2d2e"
active = "#3c3c3c"
focus = "#007acc"
selection = "rgba(173, 214, 255, 0.3)"
"##;

        let dark_theme_path = themes_dir.join("dark.toml");
        tokio::fs::write(&dark_theme_path, dark_theme_content)
            .await
            .map_err(|err| {
                ThemeConfigError::Internal(format!(
                    "Failed to create default dark theme file {}: {}",
                    dark_theme_path.display(),
                    err
                ))
            })?;

        let light_theme_content = r##"[theme]
name = "light"
theme_type = "light"

[theme.ansi]
black = "#24292e"
red = "#d73a49"
green = "#28a745"
yellow = "#ffd33d"
blue = "#0366d6"
magenta = "#ea4aaa"
cyan = "#17a2b8"
white = "#f6f8fa"

[theme.bright]
black = "#586069"
red = "#cb2431"
green = "#22863a"
yellow = "#b08800"
blue = "#005cc5"
magenta = "#e36209"
cyan = "#0598bc"
white = "#fafbfc"

[theme.syntax]
comment = "#6a737d"
string = "#032f62"
number = "#005cc5"
keyword = "#d73a49"
function = "#6f42c1"
variable = "#e36209"
type_name = "#005cc5"
operator = "#d73a49"

[theme.ui]
# 浅色主题的背景色层次
bg_100 = "#ffffff"
bg_200 = "#fafafa"
bg_300 = "#f5f5f5"
bg_400 = "#f0f0f0"
bg_500 = "#e8e8e8"
bg_600 = "#e0e0e0"
bg_700 = "#d8d8d8"

# 浅色主题的边框层次
border_200 = "rgba(0, 0, 0, 0.08)"
border_300 = "rgba(0, 0, 0, 0.12)"
border_400 = "rgba(0, 0, 0, 0.16)"

# 浅色主题的文本层次
text_100 = "#000000"
text_200 = "#1a1a1a"
text_300 = "#333333"
text_400 = "#666666"
text_500 = "#999999"

# 状态颜色
primary = "#0366d6"
primary_hover = "#005cc5"
primary_alpha = "rgba(3, 102, 214, 0.1)"
success = "#28a745"
warning = "#ffc107"
error = "#dc3545"
info = "#17a2b8"

# 交互状态
hover = "#e8e8e8"
active = "#e0e0e0"
focus = "#0366d6"
selection = "rgba(3, 102, 214, 0.3)"
"##;

        let light_theme_path = themes_dir.join("light.toml");
        tokio::fs::write(&light_theme_path, light_theme_content)
            .await
            .map_err(|err| {
                ThemeConfigError::Internal(format!(
                    "Failed to create default light theme file {}: {}",
                    light_theme_path.display(),
                    err
                ))
            })?;

        info!("Default theme files created successfully");
        Ok(())
    }

    /// 确保默认主题存在（仅在没有任何主题文件时作为后备方案）
    async fn ensure_default_themes_exist(&self, themes_dir: &Path) -> ThemeConfigResult<()> {
        let dark_theme_path = themes_dir.join("dark.toml");
        let light_theme_path = themes_dir.join("light.toml");

        if !dark_theme_path.exists() || !light_theme_path.exists() {
            self.create_default_theme_files(themes_dir).await?;
        }

        Ok(())
    }

    /// 根据实际存在的主题文件动态分类主题
    /// 所有在 themes 目录中找到的主题文件都被视为可用主题
    async fn categorize_themes(
        &self,
        themes: Vec<ThemeIndexEntry>,
    ) -> (Vec<ThemeIndexEntry>, Vec<ThemeIndexEntry>) {
        let mut builtin_themes = Vec::new();
        let custom_themes = Vec::new();

        for mut theme in themes {
            // 将所有找到的主题都标记为内置主题
            theme.builtin = true;
            builtin_themes.push(theme);
        }

        (builtin_themes, custom_themes)
    }
}

// 主题验证相关结构

/// 主题验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeValidationResult {
    /// 验证是否通过
    pub is_valid: bool,

    /// 错误信息列表
    pub errors: Vec<String>,

    /// 警告信息列表
    pub warnings: Vec<String>,
}

/// 主题验证器
pub struct ThemeValidator;

impl ThemeValidator {
    /// 验证主题
    pub fn validate_theme(theme: &Theme) -> ThemeValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 验证主题名称
        if theme.name.trim().is_empty() {
            errors.push("Theme name cannot be empty".to_string());
        }

        // 验证主题类型
        if !matches!(
            theme.theme_type,
            ThemeType::Light | ThemeType::Dark | ThemeType::Auto
        ) {
            errors.push(format!("Invalid theme type: {:?}", theme.theme_type));
        }

        // 验证主题数据
        Self::validate_theme_data(theme, &mut errors, &mut warnings);

        ThemeValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// 验证主题数据
    fn validate_theme_data(theme: &Theme, errors: &mut Vec<String>, warnings: &mut [String]) {
        // 验证 ANSI 颜色
        Self::validate_ansi_colors(&theme.ansi, "ansi", errors);
        Self::validate_ansi_colors(&theme.bright, "bright", errors);

        // 验证语法高亮
        Self::validate_syntax_highlight(&theme.syntax, errors, warnings);

        // 验证UI颜色
        Self::validate_ui_colors(&theme.ui, errors);
    }

    /// 验证 ANSI 颜色
    fn validate_ansi_colors(ansi: &AnsiColors, prefix: &str, errors: &mut Vec<String>) {
        let ansi_fields = [
            ("black", &ansi.black),
            ("red", &ansi.red),
            ("green", &ansi.green),
            ("yellow", &ansi.yellow),
            ("blue", &ansi.blue),
            ("magenta", &ansi.magenta),
            ("cyan", &ansi.cyan),
            ("white", &ansi.white),
        ];

        for (field_name, color_value) in ansi_fields.iter() {
            if !Self::is_valid_color(color_value) {
                errors.push(format!(
                    "Invalid {} color {}: {}",
                    prefix, field_name, color_value
                ));
            }
        }
    }

    /// 验证语法高亮
    fn validate_syntax_highlight(
        syntax: &SyntaxHighlight,
        errors: &mut Vec<String>,
        _warnings: &mut [String],
    ) {
        let syntax_fields = [
            ("keyword", &syntax.keyword),
            ("string", &syntax.string),
            ("comment", &syntax.comment),
            ("number", &syntax.number),
            ("function", &syntax.function),
            ("variable", &syntax.variable),
            ("type_name", &syntax.type_name),
            ("operator", &syntax.operator),
        ];

        for (field_name, color_value) in syntax_fields.iter() {
            if !Self::is_valid_color(color_value) {
                errors.push(format!(
                    "Invalid syntax highlight color {}: {}",
                    field_name, color_value
                ));
            }
        }
    }

    /// 验证UI颜色 - 支持新的层次系统
    fn validate_ui_colors(ui: &UIColors, errors: &mut Vec<String>) {
        let ui_fields = [
            // 背景色层次
            ("bg_100", &ui.bg_100),
            ("bg_200", &ui.bg_200),
            ("bg_300", &ui.bg_300),
            ("bg_400", &ui.bg_400),
            ("bg_500", &ui.bg_500),
            ("bg_600", &ui.bg_600),
            ("bg_700", &ui.bg_700),
            // 边框层次
            ("border_200", &ui.border_200),
            ("border_300", &ui.border_300),
            ("border_400", &ui.border_400),
            // 文本层次
            ("text_100", &ui.text_100),
            ("text_200", &ui.text_200),
            ("text_300", &ui.text_300),
            ("text_400", &ui.text_400),
            ("text_500", &ui.text_500),
            // 状态颜色
            ("primary", &ui.primary),
            ("primary_hover", &ui.primary_hover),
            ("primary_alpha", &ui.primary_alpha),
            ("success", &ui.success),
            ("warning", &ui.warning),
            ("error", &ui.error),
            ("info", &ui.info),
            // 交互状态
            ("hover", &ui.hover),
            ("active", &ui.active),
            ("focus", &ui.focus),
            ("selection", &ui.selection),
        ];

        for (field_name, color_value) in ui_fields.iter() {
            if !Self::is_valid_color(color_value) {
                errors.push(format!("Invalid UI color {}: {}", field_name, color_value));
            }
        }
    }

    /// 验证颜色值格式
    fn is_valid_color(color: &str) -> bool {
        if color.is_empty() {
            return false;
        }

        // 支持十六进制颜色 (#RGB, #RRGGBB, #RRGGBBAA)
        if let Some(hex_part) = color.strip_prefix('#') {
            if hex_part.len() == 3 || hex_part.len() == 6 || hex_part.len() == 8 {
                return hex_part.chars().all(|c| c.is_ascii_hexdigit());
            }
        }

        // 支持 RGB/RGBA 格式
        if color.starts_with("rgb(") || color.starts_with("rgba(") {
            return true; // 简化验证，实际应用中可以更严格
        }

        // 支持 HSL/HSLA 格式
        if color.starts_with("hsl(") || color.starts_with("hsla(") {
            return true; // 简化验证
        }

        // 支持命名颜色
        matches!(
            color.to_lowercase().as_str(),
            "black"
                | "white"
                | "red"
                | "green"
                | "blue"
                | "yellow"
                | "cyan"
                | "magenta"
                | "transparent"
                | "inherit"
                | "initial"
                | "unset"
                | "gray"
                | "grey"
                | "orange"
                | "purple"
                | "pink"
                | "brown"
                | "lime"
                | "navy"
                | "teal"
                | "silver"
                | "maroon"
                | "olive"
                | "aqua"
                | "fuchsia"
        )
    }
}
