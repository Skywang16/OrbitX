/*!
 * 文件系统管理模块
 *
 * 提供统一的文件系统操作接口，包括文件读写、备份、原子操作等
 * 支持跨平台文件操作和错误恢复
 */

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::paths::StoragePaths;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 文件系统管理器选项
#[derive(Debug, Clone)]
pub struct FileSystemOptions {
    /// 是否启用备份
    pub backup_enabled: bool,
    /// 备份保留数量
    pub backup_count: usize,
    /// 是否使用原子写入
    pub atomic_write: bool,
    /// 文件权限（Unix系统）
    pub file_permissions: Option<u32>,
    /// 目录权限（Unix系统）
    pub dir_permissions: Option<u32>,
}

impl Default for FileSystemOptions {
    fn default() -> Self {
        Self {
            backup_enabled: true,
            backup_count: 5,
            atomic_write: true,
            file_permissions: Some(0o644),
            dir_permissions: Some(0o755),
        }
    }
}

/// 文件系统管理器
pub struct FileSystemManager {
    paths: StoragePaths,
    options: FileSystemOptions,
}

impl FileSystemManager {
    /// 创建新的文件系统管理器
    pub fn new(paths: StoragePaths, options: FileSystemOptions) -> Self {
        Self { paths, options }
    }

    /// 确保所有必要的目录存在
    pub async fn initialize(&self) -> StorageResult<()> {
        self.paths.ensure_directories()?;
        log::info!("文件系统管理器初始化完成");
        Ok(())
    }

    /// 同步读取文件内容
    pub fn read_file_sync(&self, path: &Path) -> StorageResult<Vec<u8>> {
        fs::read(path).map_err(|e| {
            StorageError::filesystem_error(format!("读取文件失败: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 异步读取文件内容
    pub async fn read_file(&self, path: &Path) -> StorageResult<Vec<u8>> {
        async_fs::read(path).await.map_err(|e| {
            StorageError::filesystem_error(format!("读取文件失败: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 同步读取文件内容为字符串
    pub fn read_file_to_string_sync(&self, path: &Path) -> StorageResult<String> {
        fs::read_to_string(path).map_err(|e| {
            StorageError::filesystem_error(format!("读取文件失败: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 异步读取文件内容为字符串
    pub async fn read_file_to_string(&self, path: &Path) -> StorageResult<String> {
        async_fs::read_to_string(path).await.map_err(|e| {
            StorageError::filesystem_error(format!("读取文件失败: {}", e), Some(path.to_path_buf()))
        })
    }

    /// 同步写入文件内容
    pub fn write_file_sync(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        if self.options.atomic_write {
            self.atomic_write_sync(path, content)
        } else {
            self.direct_write_sync(path, content)
        }
    }

    /// 异步写入文件内容
    pub async fn write_file(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        if self.options.atomic_write {
            self.atomic_write(path, content).await
        } else {
            self.direct_write(path, content).await
        }
    }

    /// 同步写入字符串到文件
    pub fn write_string_sync(&self, path: &Path, content: &str) -> StorageResult<()> {
        self.write_file_sync(path, content.as_bytes())
    }

    /// 异步写入字符串到文件
    pub async fn write_string(&self, path: &Path, content: &str) -> StorageResult<()> {
        self.write_file(path, content.as_bytes()).await
    }

    /// 原子写入（同步）
    fn atomic_write_sync(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        // 创建临时文件
        let temp_path = self.get_temp_path(path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                StorageError::filesystem_error(
                    format!("创建父目录失败: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 写入临时文件
        fs::write(&temp_path, content).map_err(|e| {
            StorageError::filesystem_error(
                format!("写入临时文件失败: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        // 设置文件权限
        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            fs::set_permissions(&temp_path, perms).map_err(|e| {
                StorageError::filesystem_error(
                    format!("设置文件权限失败: {}", e),
                    Some(temp_path.clone()),
                )
            })?;
        }

        // 创建备份
        if self.options.backup_enabled && path.exists() {
            self.create_backup_sync(path)?;
        }

        // 原子移动
        fs::rename(&temp_path, path).map_err(|e| {
            // 清理临时文件
            let _ = fs::remove_file(&temp_path);
            StorageError::filesystem_error(format!("原子移动失败: {}", e), Some(path.to_path_buf()))
        })?;

        Ok(())
    }

    /// 原子写入（异步）
    async fn atomic_write(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        // 创建临时文件
        let temp_path = self.get_temp_path(path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::filesystem_error(
                    format!("创建父目录失败: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 写入临时文件
        async_fs::write(&temp_path, content).await.map_err(|e| {
            StorageError::filesystem_error(
                format!("写入临时文件失败: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        // 设置文件权限
        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            async_fs::set_permissions(&temp_path, perms)
                .await
                .map_err(|e| {
                    StorageError::filesystem_error(
                        format!("设置文件权限失败: {}", e),
                        Some(temp_path.clone()),
                    )
                })?;
        }

        // 创建备份
        if self.options.backup_enabled && async_fs::try_exists(path).await.unwrap_or(false) {
            self.create_backup(path).await?;
        }

        // 原子移动
        async_fs::rename(&temp_path, path).await.map_err(|e| {
            // 清理临时文件
            let _ = std::fs::remove_file(&temp_path);
            StorageError::filesystem_error(format!("原子移动失败: {}", e), Some(path.to_path_buf()))
        })?;

        Ok(())
    }

    /// 直接写入（同步）
    fn direct_write_sync(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                StorageError::filesystem_error(
                    format!("创建父目录失败: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 创建备份
        if self.options.backup_enabled && path.exists() {
            self.create_backup_sync(path)?;
        }

        // 直接写入
        fs::write(path, content).map_err(|e| {
            StorageError::filesystem_error(format!("写入文件失败: {}", e), Some(path.to_path_buf()))
        })?;

        // 设置文件权限
        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            fs::set_permissions(path, perms).map_err(|e| {
                StorageError::filesystem_error(
                    format!("设置文件权限失败: {}", e),
                    Some(path.to_path_buf()),
                )
            })?;
        }

        Ok(())
    }

    /// 直接写入（异步）
    async fn direct_write(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::filesystem_error(
                    format!("创建父目录失败: {}", e),
                    Some(parent.to_path_buf()),
                )
            })?;
        }

        // 创建备份
        if self.options.backup_enabled && async_fs::try_exists(path).await.unwrap_or(false) {
            self.create_backup(path).await?;
        }

        // 直接写入
        async_fs::write(path, content).await.map_err(|e| {
            StorageError::filesystem_error(format!("写入文件失败: {}", e), Some(path.to_path_buf()))
        })?;

        // 设置文件权限
        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            async_fs::set_permissions(path, perms).await.map_err(|e| {
                StorageError::filesystem_error(
                    format!("设置文件权限失败: {}", e),
                    Some(path.to_path_buf()),
                )
            })?;
        }

        Ok(())
    }

    /// 创建备份（同步）
    fn create_backup_sync(&self, path: &Path) -> StorageResult<PathBuf> {
        let backup_path = self.get_backup_path(path);

        fs::copy(path, &backup_path).map_err(|e| {
            StorageError::filesystem_error(format!("创建备份失败: {}", e), Some(path.to_path_buf()))
        })?;

        // 清理旧备份
        self.cleanup_old_backups_sync(path)?;

        log::info!("创建备份: {} -> {}", path.display(), backup_path.display());
        Ok(backup_path)
    }

    /// 创建备份（异步）
    async fn create_backup(&self, path: &Path) -> StorageResult<PathBuf> {
        let backup_path = self.get_backup_path(path);

        async_fs::copy(path, &backup_path).await.map_err(|e| {
            StorageError::filesystem_error(format!("创建备份失败: {}", e), Some(path.to_path_buf()))
        })?;

        // 清理旧备份
        self.cleanup_old_backups(path).await?;

        log::info!("创建备份: {} -> {}", path.display(), backup_path.display());
        Ok(backup_path)
    }

    /// 清理旧备份（同步）
    fn cleanup_old_backups_sync(&self, original_path: &Path) -> StorageResult<()> {
        let backup_prefix = self.get_backup_prefix(original_path);
        let mut backups = Vec::new();

        // 收集所有备份文件
        if let Ok(entries) = fs::read_dir(&self.paths.backups_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(&backup_prefix) {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                backups.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // 按修改时间排序（最新的在前）
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除超出保留数量的备份
        for (path, _) in backups.iter().skip(self.options.backup_count) {
            if let Err(e) = fs::remove_file(path) {
                log::warn!("删除旧备份失败: {} - {}", path.display(), e);
            } else {
                log::info!("删除旧备份: {}", path.display());
            }
        }

        Ok(())
    }

    /// 清理旧备份（异步）
    async fn cleanup_old_backups(&self, original_path: &Path) -> StorageResult<()> {
        let backup_prefix = self.get_backup_prefix(original_path);
        let mut backups = Vec::new();

        // 收集所有备份文件
        if let Ok(mut entries) = async_fs::read_dir(&self.paths.backups_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(&backup_prefix) {
                        if let Ok(metadata) = entry.metadata().await {
                            if let Ok(modified) = metadata.modified() {
                                backups.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // 按修改时间排序（最新的在前）
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除超出保留数量的备份
        for (path, _) in backups.iter().skip(self.options.backup_count) {
            if let Err(e) = async_fs::remove_file(path).await {
                log::warn!("删除旧备份失败: {} - {}", path.display(), e);
            } else {
                log::info!("删除旧备份: {}", path.display());
            }
        }

        Ok(())
    }

    /// 检查文件是否存在
    pub async fn exists(&self, path: &Path) -> bool {
        async_fs::try_exists(path).await.unwrap_or(false)
    }

    /// 检查文件是否存在（同步）
    pub fn exists_sync(&self, path: &Path) -> bool {
        path.exists()
    }

    /// 删除文件
    pub async fn remove_file(&self, path: &Path) -> StorageResult<()> {
        if self.exists(path).await {
            async_fs::remove_file(path).await.map_err(|e| {
                StorageError::filesystem_error(
                    format!("删除文件失败: {}", e),
                    Some(path.to_path_buf()),
                )
            })?;
            log::info!("删除文件: {}", path.display());
        }
        Ok(())
    }

    /// 删除文件（同步）
    pub fn remove_file_sync(&self, path: &Path) -> StorageResult<()> {
        if self.exists_sync(path) {
            fs::remove_file(path).map_err(|e| {
                StorageError::filesystem_error(
                    format!("删除文件失败: {}", e),
                    Some(path.to_path_buf()),
                )
            })?;
            log::info!("删除文件: {}", path.display());
        }
        Ok(())
    }

    /// 获取临时文件路径
    fn get_temp_path(&self, path: &Path) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name() {
                return parent.join(format!(".{}.tmp.{}", filename.to_string_lossy(), timestamp));
            }
        }

        path.with_extension(format!("tmp.{}", timestamp))
    }

    /// 获取备份文件路径
    fn get_backup_path(&self, path: &Path) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let backup_prefix = self.get_backup_prefix(path);
        let backup_filename = format!("{}.{}.bak", backup_prefix, timestamp);

        self.paths.backup_file(&backup_filename)
    }

    /// 获取备份文件前缀
    fn get_backup_prefix(&self, path: &Path) -> String {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            filename.replace('.', "_")
        } else {
            "unknown".to_string()
        }
    }
}
