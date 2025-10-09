/*!
 * 存储路径管理模块
 *
 * 提供统一的路径管理功能，包括配置路径、状态路径、数据路径等
 * 支持跨平台路径处理和路径验证
 */

use crate::storage::error::{StoragePathsError, StoragePathsResult};
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

    /// 备份目录
    pub backups_dir: PathBuf,
    /// 日志目录
    pub logs_dir: PathBuf,
}

impl StoragePaths {
    /// 创建新的路径管理器
    pub fn new(app_dir: PathBuf) -> StoragePathsResult<Self> {
        let config_dir = app_dir.join(super::CONFIG_DIR_NAME);
        let state_dir = app_dir.join(super::STATE_DIR_NAME);
        let data_dir = app_dir.join(super::DATA_DIR_NAME);

        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = Self {
            app_dir,
            config_dir,
            state_dir,
            data_dir,

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

    /// 获取日志文件路径
    pub fn log_file(&self, filename: &str) -> PathBuf {
        self.logs_dir.join(filename)
    }

    /// 确保所有目录存在
    pub fn ensure_directories(&self) -> StoragePathsResult<()> {
        let directories = [
            &self.app_dir,
            &self.config_dir,
            &self.state_dir,
            &self.data_dir,
            &self.backups_dir,
            &self.logs_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                fs::create_dir_all(dir).map_err(|e| {
                    StoragePathsError::directory_create(dir.to_path_buf(), e)
                })?;
                // 目录创建成功
            }
        }

        Ok(())
    }

    /// 验证路径的有效性
    pub fn validate(&self) -> StoragePathsResult<()> {
        if !self.app_dir.exists() {
            fs::create_dir_all(&self.app_dir)
                .map_err(|e| StoragePathsError::directory_create(self.app_dir.clone(), e))?;
        }

        if let Err(e) = fs::metadata(&self.app_dir) {
            return Err(StoragePathsError::directory_access(
                self.app_dir.clone(),
                e,
            ));
        }

        Ok(())
    }

    /// 获取目录大小（字节）
    pub fn get_directory_size(&self, dir: &Path) -> StoragePathsResult<u64> {
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
            .map_err(|e| StoragePathsError::directory_size(dir.to_path_buf(), e))?;

        Ok(total_size)
    }
}

/// 存储路径构建器
pub struct StoragePathsBuilder {
    app_dir: Option<PathBuf>,
    custom_config_dir: Option<PathBuf>,
    custom_state_dir: Option<PathBuf>,
    custom_data_dir: Option<PathBuf>,
}

impl StoragePathsBuilder {
    pub fn new() -> Self {
        Self {
            app_dir: None,
            custom_config_dir: None,
            custom_state_dir: None,
            custom_data_dir: None,
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

    pub fn build(self) -> StoragePathsResult<StoragePaths> {
        let Some(app_dir) = self.app_dir else {
            return Err(StoragePathsError::AppDirectoryMissing);
        };

        let config_dir = self
            .custom_config_dir
            .unwrap_or_else(|| app_dir.join(super::CONFIG_DIR_NAME));
        let state_dir = self
            .custom_state_dir
            .unwrap_or_else(|| app_dir.join(super::STATE_DIR_NAME));
        let data_dir = self
            .custom_data_dir
            .unwrap_or_else(|| app_dir.join(super::DATA_DIR_NAME));

        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = StoragePaths {
            app_dir,
            config_dir,
            state_dir,
            data_dir,

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
