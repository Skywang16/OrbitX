/*!
 * 主题系统核心模块
 *
 * 提供主题管理功能，包括主题加载、验证、索引管理和缓存。
 * 支持内置主题和用户自定义主题的统一管理。
 */

use super::types::{AnsiColors, ColorScheme, SyntaxHighlight, Theme, ThemeType, UIColors};
use crate::storage::cache::UnifiedCache;
use crate::{config::paths::ConfigPaths, utils::error::AppResult};
use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Instant, SystemTime},
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

// ============================================================================
// 主题系统数据结构
// ============================================================================

/// 主题文件包装结构 - 用于解析嵌套的 [theme] 格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileWrapper {
    /// 主题数据
    pub theme: Theme,
}

/// 主题索引条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeIndexEntry {
    /// 主题名称
    pub name: String,

    /// 文件名
    pub file: String,

    /// 主题类型
    #[serde(rename = "type")]
    pub theme_type: String,

    /// 是否为内置主题
    pub builtin: bool,

    /// 文件大小（字节）
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

// ============================================================================
// 主题管理器核心结构
// ============================================================================

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
    ) -> AppResult<Self> {
        let manager = Self {
            paths,
            index: Arc::new(RwLock::new(None)),
            cache,
            options,
            last_index_refresh: Arc::new(Mutex::new(None)),
        };

        // 初始化主题目录
        manager.ensure_theme_directories().await?;

        // 创建内置主题文件（如果不存在）
        manager.create_builtin_themes().await?;

        // 加载主题索引
        manager.load_theme_index().await?;

        info!("主题管理器初始化完成");
        Ok(manager)
    }

    /// 确保主题目录存在
    async fn ensure_theme_directories(&self) -> AppResult<()> {
        let themes_dir = self.paths.themes_dir();

        if !themes_dir.exists() {
            fs::create_dir_all(themes_dir)
                .with_context(|| format!("无法创建主题目录: {}", themes_dir.display()))?;
            info!("创建主题目录: {}", themes_dir.display());
        }

        // 创建内置主题子目录
        let builtin_dir = themes_dir.join("builtin");
        if !builtin_dir.exists() {
            fs::create_dir_all(&builtin_dir)
                .with_context(|| format!("无法创建内置主题目录: {}", builtin_dir.display()))?;
            info!("创建内置主题目录: {}", builtin_dir.display());
        }

        Ok(())
    }

    /// 加载主题索引
    async fn load_theme_index(&self) -> AppResult<()> {
        let index_path = self.paths.themes_dir().join("index.toml");

        let theme_index = if index_path.exists() {
            self.load_index_from_file(&index_path).await?
        } else {
            self.create_default_index().await?
        };

        // 更新索引
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow!("无法获取索引写锁: {}", e))?;
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
    async fn load_index_from_file(&self, index_path: &Path) -> AppResult<ThemeIndex> {
        let content = tokio::fs::read_to_string(index_path)
            .await
            .with_context(|| format!("无法读取主题索引文件: {}", index_path.display()))?;

        let index: ThemeIndex = toml::from_str(&content)
            .with_context(|| format!("无法解析主题索引文件: {}", index_path.display()))?;

        debug!("成功加载主题索引，包含 {} 个主题", index.total_themes);
        Ok(index)
    }

    /// 创建默认索引
    async fn create_default_index(&self) -> AppResult<ThemeIndex> {
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
    async fn save_index_to_file(&self, index: &ThemeIndex) -> AppResult<()> {
        let index_path = self.paths.themes_dir().join("index.toml");

        let content = toml::to_string_pretty(index).with_context(|| "无法序列化主题索引")?;

        tokio::fs::write(&index_path, content)
            .await
            .with_context(|| format!("无法写入主题索引文件: {}", index_path.display()))?;

        debug!("主题索引已保存到: {}", index_path.display());
        Ok(())
    }

    /// 加载主题
    ///
    /// # Arguments
    /// * `theme_name` - 主题名称
    ///
    /// # Returns
    /// 返回主题数据
    pub async fn load_theme(&self, theme_name: &str) -> AppResult<Theme> {
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
    async fn load_theme_from_file(&self, theme_name: &str) -> AppResult<Theme> {
        let theme_path = self.get_theme_file_path(theme_name).await?;

        let content = tokio::fs::read_to_string(&theme_path)
            .await
            .with_context(|| format!("无法读取主题文件: {}", theme_path.display()))?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content).map_err(|e| {
            anyhow!(
                "无法解析主题文件: {} \nTOML解析错误: {} \n文件内容:\n{}",
                theme_path.display(),
                e,
                content
            )
        })?;

        let theme = theme_wrapper.theme;

        // 验证主题
        let validation_result = ThemeValidator::validate_theme(&theme);
        if !validation_result.is_valid {
            bail!("主题验证失败: {:?}", validation_result.errors);
        }

        if !validation_result.warnings.is_empty() {
            warn!(
                "主题 {} 存在警告: {:?}",
                theme_name, validation_result.warnings
            );
        }

        Ok(theme)
    }

    /// 获取主题文件路径
    async fn get_theme_file_path(&self, theme_name: &str) -> AppResult<PathBuf> {
        let themes_dir = self.paths.themes_dir();

        // 直接在 themes 目录下查找主题文件
        let theme_path = themes_dir.join(format!("{}.toml", theme_name));
        if theme_path.exists() {
            return Ok(theme_path);
        }

        bail!(
            "主题文件不存在: {} (搜索路径: {})",
            theme_name,
            themes_dir.display()
        );
    }

    /// 获取所有可用主题列表
    pub async fn list_themes(&self) -> AppResult<Vec<ThemeIndexEntry>> {
        // 首先尝试获取现有索引
        let themes = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow!("无法获取索引读锁: {}", e))?;

            if let Some(ref theme_index) = *index {
                let mut themes = Vec::new();
                themes.extend(theme_index.builtin_themes.clone());
                themes.extend(theme_index.custom_themes.clone());

                debug!(
                    "获取主题列表: {} 个内置主题, {} 个自定义主题",
                    theme_index.builtin_themes.len(),
                    theme_index.custom_themes.len()
                );

                Some(themes)
            } else {
                None
            }
        };

        // 如果索引未初始化，重新加载
        if themes.is_none() {
            warn!("主题索引未初始化，尝试重新加载");
            self.load_theme_index().await?;

            let index = self
                .index
                .read()
                .map_err(|e| anyhow!("无法获取索引读锁: {}", e))?;

            if let Some(ref theme_index) = *index {
                let mut themes = Vec::new();
                themes.extend(theme_index.builtin_themes.clone());
                themes.extend(theme_index.custom_themes.clone());
                return Ok(themes);
            } else {
                bail!("主题索引重新加载后仍未初始化");
            }
        }

        let themes = themes.unwrap();

        // 如果主题列表为空，尝试刷新索引
        if themes.is_empty() {
            warn!("主题列表为空，尝试刷新索引");
            self.refresh_index().await?;

            let index = self
                .index
                .read()
                .map_err(|e| anyhow!("无法获取索引读锁: {}", e))?;

            if let Some(ref theme_index) = *index {
                let mut refreshed_themes = Vec::new();
                refreshed_themes.extend(theme_index.builtin_themes.clone());
                refreshed_themes.extend(theme_index.custom_themes.clone());
                info!("刷新后获取到 {} 个主题", refreshed_themes.len());
                return Ok(refreshed_themes);
            }
        }

        Ok(themes)
    }

    /// 获取主题索引
    pub async fn get_theme_index(&self) -> AppResult<ThemeIndex> {
        let index = self
            .index
            .read()
            .map_err(|e| anyhow!("无法获取索引读锁: {}", e))?;

        if let Some(ref theme_index) = *index {
            Ok(theme_index.clone())
        } else {
            bail!("主题索引未初始化");
        }
    }

    /// 刷新主题索引
    pub async fn refresh_index(&self) -> AppResult<()> {
        info!("开始刷新主题索引");

        // 扫描主题目录下的所有主题文件
        let themes_dir = self.paths.themes_dir();
        let all_themes = self.scan_theme_directory(themes_dir, true).await?;

        // 创建新索引
        let total_themes = all_themes.len();
        let new_index = ThemeIndex {
            version: "1.0.0".to_string(),
            last_updated: SystemTime::now(),
            total_themes,
            builtin_themes: all_themes,
            custom_themes: Vec::new(),
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
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow!("无法获取索引写锁: {}", e))?;
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
    ) -> AppResult<Vec<ThemeIndexEntry>> {
        let mut themes = Vec::new();

        if !dir.exists() {
            return Ok(themes);
        }

        let mut entries = tokio::fs::read_dir(dir)
            .await
            .with_context(|| format!("无法读取目录: {}", dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("无法读取目录条目: {}", dir.display()))?
        {
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
                            warn!("无法创建主题索引条目 {}: {}", file_name, e);
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
    ) -> AppResult<ThemeIndexEntry> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;

        let file_size = metadata.len();
        let last_modified = metadata.modified().ok();

        // 尝试读取主题文件以获取元数据
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("无法读取主题文件: {}", path.display()))?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content)
            .with_context(|| format!("无法解析主题文件: {}", path.display()))?;

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
    pub async fn create_builtin_themes(&self) -> AppResult<()> {
        let themes_dir = self.paths.themes_dir();

        // 确保themes目录存在
        if !themes_dir.exists() {
            fs::create_dir_all(themes_dir)
                .with_context(|| format!("无法创建主题目录: {}", themes_dir.display()))?;
            info!("创建主题目录: {}", themes_dir.display());
        }

        // 尝试从资源目录复制主题文件
        self.copy_themes_from_resources(themes_dir).await?;

        // 如果资源复制失败，回退到创建默认主题
        self.ensure_default_themes_exist(themes_dir).await?;

        // 内置主题文件检查完成

        // 刷新主题索引以确保新创建的主题被正确识别
        if let Err(e) = self.refresh_index().await {
            warn!("刷新主题索引失败: {}", e);
        } else {
            debug!("主题索引已刷新");
        }

        Ok(())
    }

    /// 扫描主题目录并加载现有主题文件
    #[allow(dead_code)]
    async fn scan_and_load_existing_themes(&self, themes_dir: &Path) -> AppResult<()> {
        // 检查主题目录是否存在
        if !themes_dir.exists() {
            self.create_default_theme_files(themes_dir).await?;
            return Ok(());
        }

        // 扫描现有的 TOML 主题文件
        let entries = fs::read_dir(themes_dir)
            .with_context(|| format!("无法读取主题目录: {}", themes_dir.display()))?;

        let mut theme_count = 0;
        for entry in entries {
            let entry = entry.with_context(|| "无法读取目录条目")?;
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
                            warn!("主题文件格式无效 {}: {}", path.display(), e);
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
    async fn validate_theme_file(&self, path: &Path) -> AppResult<()> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("无法读取主题文件: {}", path.display()))?;

        let theme_wrapper: ThemeFileWrapper = toml::from_str(&content)
            .with_context(|| format!("无法解析主题文件: {}", path.display()))?;

        let theme = theme_wrapper.theme;

        // 验证主题
        let validation_result = ThemeValidator::validate_theme(&theme);
        if !validation_result.is_valid {
            bail!("主题验证失败: {:?}", validation_result.errors);
        }

        Ok(())
    }

    /// 创建默认主题文件
    async fn create_default_theme_files(&self, themes_dir: &Path) -> AppResult<()> {
        info!("创建默认主题文件");

        // 创建默认的深色主题
        let dark_theme_content = r##"[theme]
name = "dark"
theme_type = "dark"

[theme.colors]
foreground = "#e6e6e6"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#3391ff"

[theme.colors.ansi]
black = "#000000"
red = "#cd3131"
green = "#0dbc79"
yellow = "#e5e510"
blue = "#2472c8"
magenta = "#bc3fbc"
cyan = "#11a8cd"
white = "#e5e5e5"

[theme.colors.bright]
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
primary = "#007acc"
secondary = "#6c757d"
success = "#28a745"
warning = "#ffc107"
error = "#dc3545"
info = "#17a2b8"
border = "#3c3c3c"
divider = "#404040"
"##;

        let dark_theme_path = themes_dir.join("dark.toml");
        tokio::fs::write(&dark_theme_path, dark_theme_content)
            .await
            .with_context(|| format!("无法创建默认深色主题文件: {}", dark_theme_path.display()))?;

        // 创建默认的浅色主题
        let light_theme_content = r##"[theme]
name = "light"
theme_type = "light"

[theme.colors]
foreground = "#24292e"
background = "#ffffff"
cursor = "#044289"
selection = "#0366d6"

[theme.colors.ansi]
black = "#24292e"
red = "#d73a49"
green = "#28a745"
yellow = "#b08800"
blue = "#0366d6"
magenta = "#ea4aaa"
cyan = "#17a2b8"
white = "#586069"

[theme.colors.bright]
black = "#6a737d"
red = "#cb2431"
green = "#22863a"
yellow = "#e36209"
blue = "#005cc5"
magenta = "#b392f0"
cyan = "#0598bc"
white = "#24292e"

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
primary = "#0366d6"
secondary = "#6c757d"
success = "#28a745"
warning = "#ffc107"
error = "#dc3545"
info = "#17a2b8"
border = "#e1e4e8"
divider = "#d1d5da"
"##;

        let light_theme_path = themes_dir.join("light.toml");
        tokio::fs::write(&light_theme_path, light_theme_content)
            .await
            .with_context(|| format!("无法创建默认浅色主题文件: {}", light_theme_path.display()))?;

        info!("默认主题文件创建完成");
        Ok(())
    }

    /// 从打包的资源目录复制主题文件
    async fn copy_themes_from_resources(&self, _themes_dir: &Path) -> AppResult<()> {
        // 实际的资源复制在应用初始化时通过 AppHandle 完成
        // 这里只是保持接口兼容性
        Ok(())
    }

    /// 确保默认主题存在
    async fn ensure_default_themes_exist(&self, themes_dir: &Path) -> AppResult<()> {
        // 检查是否存在基本的 dark 和 light 主题
        let dark_theme_path = themes_dir.join("dark.toml");
        let light_theme_path = themes_dir.join("light.toml");

        if !dark_theme_path.exists() || !light_theme_path.exists() {
            self.create_default_theme_files(themes_dir).await?;
        }

        Ok(())
    }
}

// ============================================================================
// 主题验证相关结构
// ============================================================================

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
            errors.push("主题名称不能为空".to_string());
        }

        // 验证主题类型
        if !matches!(
            theme.theme_type,
            ThemeType::Light | ThemeType::Dark | ThemeType::Auto
        ) {
            errors.push(format!("无效的主题类型: {:?}", theme.theme_type));
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
        // 验证颜色值
        Self::validate_color_scheme(&theme.colors, errors, warnings);

        // 验证语法高亮
        Self::validate_syntax_highlight(&theme.syntax, errors, warnings);

        // 验证UI颜色
        Self::validate_ui_colors(&theme.ui, errors, warnings);
    }

    /// 验证颜色方案
    fn validate_color_scheme(
        colors: &ColorScheme,
        errors: &mut Vec<String>,
        _warnings: &mut [String],
    ) {
        let color_fields = [
            ("foreground", &colors.foreground),
            ("background", &colors.background),
            ("cursor", &colors.cursor),
            ("selection", &colors.selection),
        ];

        for (field_name, color_value) in color_fields.iter() {
            if !Self::is_valid_color(color_value) {
                errors.push(format!("无效的颜色值 {}: {}", field_name, color_value));
            }
        }

        // 验证 ANSI 颜色
        Self::validate_ansi_colors(&colors.ansi, "ansi", errors);
        Self::validate_ansi_colors(&colors.bright, "bright", errors);
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
                    "无效的{}颜色 {}: {}",
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
                    "无效的语法高亮颜色 {}: {}",
                    field_name, color_value
                ));
            }
        }
    }

    /// 验证UI颜色
    fn validate_ui_colors(ui: &UIColors, errors: &mut Vec<String>, _warnings: &mut [String]) {
        let ui_fields = [
            ("primary", &ui.primary),
            ("secondary", &ui.secondary),
            ("success", &ui.success),
            ("warning", &ui.warning),
            ("error", &ui.error),
            ("info", &ui.info),
            ("border", &ui.border),
            ("divider", &ui.divider),
        ];

        for (field_name, color_value) in ui_fields.iter() {
            if !Self::is_valid_color(color_value) {
                errors.push(format!("无效的UI颜色 {}: {}", field_name, color_value));
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
