/*!
 * 配置系统路径管理模块
 *
 * 提供统一的配置文件路径管理，支持跨平台路径解析和目录创建。
 */

use crate::utils::error::AppResult;
use anyhow::{anyhow, bail, Context};
use std::path::{Path, PathBuf};

/// 配置路径管理器
///
/// 负责管理所有配置相关的文件和目录路径，提供跨平台的路径解析功能。
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// 应用程序数据目录
    app_data_dir: PathBuf,

    /// 配置目录
    config_dir: PathBuf,

    /// 主题目录
    themes_dir: PathBuf,

    /// 备份目录
    backups_dir: PathBuf,

    /// 缓存目录
    cache_dir: PathBuf,

    /// 日志目录
    logs_dir: PathBuf,
}

impl ConfigPaths {
    /// 创建新的配置路径管理器
    ///
    /// 根据当前平台自动确定配置目录位置。
    ///
    /// # 错误
    ///
    /// 如果无法确定用户目录或创建必要的目录，将返回错误。
    pub fn new() -> AppResult<Self> {
        let app_data_dir = Self::get_app_data_dir()?;
        Self::with_app_data_dir(app_data_dir)
    }

    /// 使用自定义应用数据目录创建配置路径管理器
    ///
    /// # 参数
    ///
    /// * `app_data_dir` - 自定义的应用数据目录路径
    ///
    /// # 错误
    ///
    /// 如果无法创建必要的目录，将返回错误。
    pub fn with_app_data_dir<P: AsRef<Path>>(app_data_dir: P) -> AppResult<Self> {
        let app_data_dir = app_data_dir.as_ref().to_path_buf();

        let config_dir = app_data_dir.join(crate::config::CONFIG_DIR_NAME);
        let themes_dir = app_data_dir.join(crate::config::THEMES_DIR_NAME);
        let backups_dir = app_data_dir.join(crate::config::BACKUPS_DIR_NAME);
        let cache_dir = app_data_dir.join(crate::config::CACHE_DIR_NAME);
        let logs_dir = app_data_dir.join(crate::config::LOGS_DIR_NAME);

        let paths = Self {
            app_data_dir,
            config_dir,
            themes_dir,
            backups_dir,
            cache_dir,
            logs_dir,
        };

        // 确保所有必要的目录存在
        paths.ensure_directories_exist()?;

        Ok(paths)
    }

    /// 获取应用程序数据目录
    ///
    /// 根据不同平台返回合适的应用数据目录：
    /// - Windows: `%APPDATA%\TerminalApp`
    /// - macOS: `~/Library/Application Support/TerminalApp`
    /// - Linux: `~/.config/terminal-app`
    fn get_app_data_dir() -> AppResult<PathBuf> {
        let app_name = "TerminalApp";

        #[cfg(target_os = "windows")]
        {
            use std::env;
            let appdata = env::var("APPDATA").with_context(|| "无法获取 APPDATA 环境变量")?;
            Ok(PathBuf::from(appdata).join(app_name))
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
            Ok(home
                .join("Library")
                .join("Application Support")
                .join(app_name))
        }

        #[cfg(target_os = "linux")]
        {
            let config_dir = dirs::config_dir().ok_or_else(|| anyhow!("无法获取配置目录"))?;
            Ok(config_dir.join("terminal-app"))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
            Ok(home.join(".terminal-app"))
        }
    }

    /// 获取项目配置目录
    ///
    /// 返回项目根目录下的 config 目录路径
    #[allow(dead_code)]
    fn get_project_config_dir() -> AppResult<PathBuf> {
        // 获取当前可执行文件的目录
        let exe_path = std::env::current_exe().with_context(|| "无法获取可执行文件路径")?;

        // 在开发环境中，可执行文件在 target/debug/ 或 target/release/ 下
        // 我们需要找到项目根目录
        let mut current_dir = exe_path
            .parent()
            .ok_or_else(|| anyhow!("无法获取可执行文件目录"))?
            .to_path_buf();

        // 向上查找，直到找到包含 src-tauri 目录的项目根目录
        loop {
            if current_dir.join("src-tauri").exists() {
                return Ok(current_dir.join("config"));
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // 如果找不到项目根目录，就使用当前工作目录下的 config
                let cwd = std::env::current_dir().with_context(|| "无法获取当前工作目录")?;
                return Ok(cwd.join("config"));
            }
        }
    }

    /// 获取项目主题目录
    ///
    /// 返回项目根目录下的 config/themes 目录路径
    #[allow(dead_code)]
    fn get_project_themes_dir() -> AppResult<PathBuf> {
        let config_dir = Self::get_project_config_dir()?;
        Ok(config_dir.join("themes"))
    }

    /// 确保所有必要的目录存在
    fn ensure_directories_exist(&self) -> AppResult<()> {
        let directories = [
            &self.app_data_dir,
            &self.config_dir,
            &self.themes_dir,
            &self.backups_dir,
            &self.cache_dir,
            &self.logs_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("无法创建目录: {}", dir.display()))?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // 路径获取方法
    // ========================================================================

    /// 获取应用程序数据目录路径
    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    /// 获取配置目录路径
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// 获取主配置文件路径
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(crate::config::CONFIG_FILE_NAME)
    }

    /// 获取主题目录路径
    pub fn themes_dir(&self) -> &Path {
        &self.themes_dir
    }

    /// 获取指定主题文件路径
    pub fn theme_file<S: AsRef<str>>(&self, theme_name: S) -> PathBuf {
        self.themes_dir
            .join(format!("{}.toml", theme_name.as_ref()))
    }

    /// 获取备份目录路径
    pub fn backups_dir(&self) -> &Path {
        &self.backups_dir
    }

    /// 获取配置备份文件路径
    pub fn config_backup_file(&self, timestamp: &str) -> PathBuf {
        self.backups_dir.join(format!("config_{}.toml", timestamp))
    }

    /// 获取缓存目录路径
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// 获取配置缓存文件路径
    pub fn config_cache_file(&self) -> PathBuf {
        self.cache_dir.join("config.cache")
    }

    /// 获取主题缓存文件路径
    pub fn theme_cache_file(&self) -> PathBuf {
        self.cache_dir.join("themes.cache")
    }

    /// 获取日志目录路径
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// 获取配置日志文件路径
    pub fn config_log_file(&self) -> PathBuf {
        self.logs_dir.join("config.log")
    }

    /// 获取错误日志文件路径
    pub fn error_log_file(&self) -> PathBuf {
        self.logs_dir.join("error.log")
    }

    // ========================================================================
    // 路径验证和操作方法
    // ========================================================================

    /// 验证路径是否在允许的目录范围内
    ///
    /// # 参数
    ///
    /// * `path` - 要验证的路径
    ///
    /// # 返回
    ///
    /// 如果路径安全，返回 `Ok(())`，否则返回错误。
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let path = path.as_ref();
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("无法规范化路径: {}", path.display()))?;

        let canonical_app_dir = self
            .app_data_dir
            .canonicalize()
            .with_context(|| format!("无法规范化应用目录: {}", self.app_data_dir.display()))?;

        if !canonical_path.starts_with(&canonical_app_dir) {
            bail!("路径不在允许的目录范围内: {}", path.display());
        }

        Ok(())
    }

    /// 检查文件是否存在
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists() && path.as_ref().is_file()
    }

    /// 检查目录是否存在
    pub fn dir_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists() && path.as_ref().is_dir()
    }

    /// 获取文件大小
    pub fn file_size<P: AsRef<Path>>(&self, path: P) -> AppResult<u64> {
        let metadata = std::fs::metadata(path.as_ref())
            .with_context(|| format!("无法获取文件元数据: {}", path.as_ref().display()))?;

        Ok(metadata.len())
    }

    /// 获取文件修改时间
    pub fn file_modified_time<P: AsRef<Path>>(&self, path: P) -> AppResult<std::time::SystemTime> {
        let metadata = std::fs::metadata(path.as_ref())
            .with_context(|| format!("无法获取文件元数据: {}", path.as_ref().display()))?;

        metadata
            .modified()
            .with_context(|| format!("无法获取文件修改时间: {}", path.as_ref().display()))
    }

    /// 创建目录
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let path = path.as_ref();

        // 验证路径安全性
        self.validate_path(path)?;

        std::fs::create_dir_all(path)
            .with_context(|| format!("无法创建目录: {}", path.display()))?;

        Ok(())
    }

    /// 删除文件
    pub fn remove_file<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let path = path.as_ref();

        // 验证路径安全性
        self.validate_path(path)?;

        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("无法删除文件: {}", path.display()))?;
        }

        Ok(())
    }

    /// 删除目录
    pub fn remove_dir<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let path = path.as_ref();

        // 验证路径安全性
        self.validate_path(path)?;

        if path.exists() {
            std::fs::remove_dir_all(path)
                .with_context(|| format!("无法删除目录: {}", path.display()))?;
        }

        Ok(())
    }

    /// 复制文件
    pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> AppResult<()> {
        let from = from.as_ref();
        let to = to.as_ref();

        // 验证路径安全性
        self.validate_path(from)?;
        self.validate_path(to)?;

        // 确保目标目录存在
        if let Some(parent) = to.parent() {
            self.create_dir(parent)?;
        }

        std::fs::copy(from, to)
            .with_context(|| format!("无法复制文件: {} -> {}", from.display(), to.display()))?;

        Ok(())
    }

    /// 移动文件
    pub fn move_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> AppResult<()> {
        let from = from.as_ref();
        let to = to.as_ref();

        // 验证路径安全性
        self.validate_path(from)?;
        self.validate_path(to)?;

        // 确保目标目录存在
        if let Some(parent) = to.parent() {
            self.create_dir(parent)?;
        }

        std::fs::rename(from, to)
            .with_context(|| format!("无法移动文件: {} -> {}", from.display(), to.display()))?;

        Ok(())
    }

    // ========================================================================
    // 便捷方法
    // ========================================================================

    /// 列出主题目录中的所有主题文件
    pub fn list_theme_files(&self) -> AppResult<Vec<PathBuf>> {
        let mut theme_files = Vec::new();

        if self.themes_dir.exists() {
            let entries = std::fs::read_dir(&self.themes_dir)
                .with_context(|| format!("无法读取主题目录: {}", self.themes_dir.display()))?;

            for entry in entries {
                let entry = entry.with_context(|| "无法读取主题目录项")?;

                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    theme_files.push(path);
                }
            }
        }

        Ok(theme_files)
    }

    /// 列出备份目录中的所有备份文件
    pub fn list_backup_files(&self) -> AppResult<Vec<PathBuf>> {
        let mut backup_files = Vec::new();

        if self.backups_dir.exists() {
            let entries = std::fs::read_dir(&self.backups_dir)
                .with_context(|| format!("无法读取备份目录: {}", self.backups_dir.display()))?;

            for entry in entries {
                let entry = entry.with_context(|| "无法读取备份目录项")?;

                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    backup_files.push(path);
                }
            }
        }

        // 按修改时间排序（最新的在前）
        backup_files.sort_by(|a, b| {
            let a_time = self.file_modified_time(a).unwrap_or(std::time::UNIX_EPOCH);
            let b_time = self.file_modified_time(b).unwrap_or(std::time::UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        Ok(backup_files)
    }

    /// 清理旧的备份文件
    pub fn cleanup_old_backups(&self, keep_count: usize) -> AppResult<()> {
        let backup_files = self.list_backup_files()?;

        if backup_files.len() > keep_count {
            for file in backup_files.iter().skip(keep_count) {
                self.remove_file(file)?;
            }
        }

        Ok(())
    }

    /// 获取目录大小
    pub fn dir_size<P: AsRef<Path>>(&self, path: P) -> AppResult<u64> {
        let path = path.as_ref();
        let mut total_size = 0;

        if path.is_dir() {
            let entries = std::fs::read_dir(path)
                .with_context(|| format!("无法读取目录: {}", path.display()))?;

            for entry in entries {
                let entry = entry.with_context(|| "无法读取目录项")?;

                let entry_path = entry.path();
                if entry_path.is_file() {
                    total_size += self.file_size(&entry_path)?;
                } else if entry_path.is_dir() {
                    total_size += self.dir_size(&entry_path)?;
                }
            }
        }

        Ok(total_size)
    }

    /// 创建用于测试的配置路径管理器
    #[cfg(test)]
    pub fn new_for_test(base_dir: PathBuf) -> Self {
        Self {
            app_data_dir: base_dir.clone(),
            config_dir: base_dir.clone(),
            themes_dir: base_dir.join("themes"),
            backups_dir: base_dir.join("backups"),
            cache_dir: base_dir.join("cache"),
            logs_dir: base_dir.join("logs"),
        }
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self::new().expect("无法创建默认配置路径")
    }
}
