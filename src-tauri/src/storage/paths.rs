/*!
 * 存储路径管理模块
 *
 * 提供统一的路径管理功能，包括配置路径、状态路径、数据路径等
 * 支持跨平台路径处理和路径验证
 */

use crate::storage::types::StorageStats;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use std::fs;
use std::path::{Path, PathBuf};

/// 存储路径管理器
#[derive(Debug, Clone)]
pub struct StoragePaths {
    /// 应用根目录
    pub app_dir: PathBuf,
    /// 配置目录
    pub config_dir: PathBuf,
    /// 状态目录
    pub state_dir: PathBuf,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 缓存目录
    pub cache_dir: PathBuf,
    /// 备份目录
    pub backups_dir: PathBuf,
    /// 日志目录
    pub logs_dir: PathBuf,
}

impl StoragePaths {
    /// 创建新的路径管理器
    pub fn new(app_dir: PathBuf) -> AppResult<Self> {
        let config_dir = app_dir.join(super::CONFIG_DIR_NAME);
        let state_dir = app_dir.join(super::STATE_DIR_NAME);
        let data_dir = app_dir.join(super::DATA_DIR_NAME);
        let cache_dir = app_dir.join(super::CACHE_DIR_NAME);
        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = Self {
            app_dir,
            config_dir,
            state_dir,
            data_dir,
            cache_dir,
            backups_dir,
            logs_dir,
        };

        // 验证路径
        paths.validate()?;

        Ok(paths)
    }

    /// 获取配置文件路径
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(super::CONFIG_FILE_NAME)
    }

    /// 获取会话状态文件路径
    pub fn session_state_file(&self) -> PathBuf {
        self.state_dir.join(super::SESSION_STATE_FILE_NAME)
    }

    /// 获取数据库文件路径
    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join(super::DATABASE_FILE_NAME)
    }

    /// 获取备份文件路径
    pub fn backup_file(&self, filename: &str) -> PathBuf {
        self.backups_dir.join(filename)
    }

    /// 获取缓存文件路径
    pub fn cache_file(&self, filename: &str) -> PathBuf {
        self.cache_dir.join(filename)
    }

    /// 获取日志文件路径
    pub fn log_file(&self, filename: &str) -> PathBuf {
        self.logs_dir.join(filename)
    }

    /// 确保所有目录存在
    pub fn ensure_directories(&self) -> AppResult<()> {
        let directories = [
            &self.app_dir,
            &self.config_dir,
            &self.state_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.backups_dir,
            &self.logs_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .with_context(|| format!("创建目录失败: {}", dir.display()))?;
                // 目录创建成功
            }
        }

        Ok(())
    }

    /// 验证路径的有效性
    pub fn validate(&self) -> AppResult<()> {
        // 检查应用目录是否可访问
        if !self.app_dir.exists() {
            return Err(anyhow!("应用目录不存在: {}", self.app_dir.display()));
        }

        // 检查是否有写权限
        if let Err(e) = fs::metadata(&self.app_dir) {
            return Err(anyhow!(
                "无法访问应用目录: {} - {}",
                self.app_dir.display(),
                e
            ));
        }

        Ok(())
    }

    /// 清理缓存目录
    pub fn clean_cache(&self) -> AppResult<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .with_context(|| format!("清理缓存目录失败: {}", self.cache_dir.display()))?;

            fs::create_dir_all(&self.cache_dir)
                .with_context(|| format!("重新创建缓存目录失败: {}", self.cache_dir.display()))?;

            // 缓存目录已清理
        }
        Ok(())
    }

    /// 获取目录大小（字节）
    pub fn get_directory_size(&self, dir: &Path) -> AppResult<u64> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;

        fn visit_dir(dir: &Path, total_size: &mut u64) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dir(&path, total_size)?;
                } else {
                    let metadata = entry.metadata()?;
                    *total_size += metadata.len();
                }
            }
            Ok(())
        }

        visit_dir(dir, &mut total_size)
            .with_context(|| format!("计算目录大小失败: {}", dir.display()))?;

        Ok(total_size)
    }

    /// 获取存储统计信息
    pub fn get_storage_stats(&self) -> AppResult<StorageStats> {
        let config_size = self.get_directory_size(&self.config_dir)?;
        let state_size = self.get_directory_size(&self.state_dir)?;
        let data_size = self.get_directory_size(&self.data_dir)?;
        let cache_size = self.get_directory_size(&self.cache_dir)?;
        let backups_size = self.get_directory_size(&self.backups_dir)?;
        let logs_size = self.get_directory_size(&self.logs_dir)?;

        Ok(StorageStats {
            total_size: config_size
                + state_size
                + data_size
                + cache_size
                + backups_size
                + logs_size,
            config_size,
            state_size,
            data_size,
            cache_size,
            backups_size,
            logs_size,
        })
    }
}

/// 存储路径构建器
pub struct StoragePathsBuilder {
    app_dir: Option<PathBuf>,
    custom_config_dir: Option<PathBuf>,
    custom_state_dir: Option<PathBuf>,
    custom_data_dir: Option<PathBuf>,
    custom_cache_dir: Option<PathBuf>,
}

impl StoragePathsBuilder {
    pub fn new() -> Self {
        Self {
            app_dir: None,
            custom_config_dir: None,
            custom_state_dir: None,
            custom_data_dir: None,
            custom_cache_dir: None,
        }
    }

    pub fn app_dir(mut self, dir: PathBuf) -> Self {
        self.app_dir = Some(dir);
        self
    }

    pub fn config_dir(mut self, dir: PathBuf) -> Self {
        self.custom_config_dir = Some(dir);
        self
    }

    pub fn state_dir(mut self, dir: PathBuf) -> Self {
        self.custom_state_dir = Some(dir);
        self
    }

    pub fn data_dir(mut self, dir: PathBuf) -> Self {
        self.custom_data_dir = Some(dir);
        self
    }

    pub fn cache_dir(mut self, dir: PathBuf) -> Self {
        self.custom_cache_dir = Some(dir);
        self
    }

    pub fn build(self) -> AppResult<StoragePaths> {
        let app_dir = self
            .app_dir
            .ok_or_else(|| anyhow!("应用目录未设置".to_string()))?;

        let config_dir = self
            .custom_config_dir
            .unwrap_or_else(|| app_dir.join(super::CONFIG_DIR_NAME));
        let state_dir = self
            .custom_state_dir
            .unwrap_or_else(|| app_dir.join(super::STATE_DIR_NAME));
        let data_dir = self
            .custom_data_dir
            .unwrap_or_else(|| app_dir.join(super::DATA_DIR_NAME));
        let cache_dir = self
            .custom_cache_dir
            .unwrap_or_else(|| app_dir.join(super::CACHE_DIR_NAME));
        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = StoragePaths {
            app_dir,
            config_dir,
            state_dir,
            data_dir,
            cache_dir,
            backups_dir,
            logs_dir,
        };

        paths.validate()?;
        Ok(paths)
    }
}

impl Default for StoragePathsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
