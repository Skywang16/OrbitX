//! 文件系统管理模块

use crate::storage::paths::StoragePaths;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use std::fs;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as tokio_fs;

/// 文件系统管理器选项
#[derive(Debug, Clone)]
pub struct FileSystemOptions {
    /// 是否启用备份
    pub backup_enabled: bool,
    /// 备份保留数量
    pub backup_count: usize,
    /// 是否使用原子写入
    pub atomic_write: bool,
    /// 文件权限
    pub file_permissions: Option<u32>,
    /// 目录权限
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
    pub async fn initialize(&self) -> AppResult<()> {
        self.paths.ensure_directories()?;
        // 初始化完成
        Ok(())
    }

    /// 同步读取文件内容
    pub fn read_file_sync(&self, path: &Path) -> AppResult<Vec<u8>> {
        fs::read(path).with_context(|| format!("读取文件失败: {}", path.display()))
    }

    /// 异步读取文件内容
    pub async fn read_file(&self, path: &Path) -> AppResult<Vec<u8>> {
        tokio_fs::read(path)
            .await
            .with_context(|| format!("读取文件失败: {}", path.display()))
    }

    /// 同步读取文件内容为字符串
    pub fn read_file_to_string_sync(&self, path: &Path) -> AppResult<String> {
        fs::read_to_string(path).with_context(|| format!("读取文件失败: {}", path.display()))
    }

    /// 异步读取文件内容为字符串
    pub async fn read_file_to_string(&self, path: &Path) -> AppResult<String> {
        tokio_fs::read_to_string(path)
            .await
            .with_context(|| format!("读取文件失败: {}", path.display()))
    }

    /// 同步写入文件内容
    pub fn write_file_sync(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        if self.options.atomic_write {
            self.atomic_write_sync(path, content)
        } else {
            self.direct_write_sync(path, content)
        }
    }

    /// 异步写入文件内容
    pub async fn write_file(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        if self.options.atomic_write {
            self.atomic_write(path, content).await
        } else {
            self.direct_write(path, content).await
        }
    }

    /// 同步写入字符串到文件
    pub fn write_string_sync(&self, path: &Path, content: &str) -> AppResult<()> {
        self.write_file_sync(path, content.as_bytes())
    }

    /// 异步写入字符串到文件
    pub async fn write_string(&self, path: &Path, content: &str) -> AppResult<()> {
        self.write_file(path, content.as_bytes()).await
    }

    /// 原子写入（同步）
    fn atomic_write_sync(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        let temp_path = self.get_temp_path(path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建父目录失败: {}", parent.display()))?;
        }

        // 写入临时文件
        fs::write(&temp_path, content)
            .with_context(|| format!("写入临时文件失败: {}", temp_path.display()))?;

        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            fs::set_permissions(&temp_path, perms)
                .with_context(|| format!("设置文件权限失败: {}", temp_path.display()))?;
        }

        if self.options.backup_enabled && path.exists() {
            self.create_backup_sync(path)?;
        }

        // 原子移动
        fs::rename(&temp_path, path).map_err(|e| {
            // 清理临时文件
            let _ = fs::remove_file(&temp_path);
            anyhow!("原子移动失败: {}", e)
        })?;

        Ok(())
    }

    /// 原子写入（异步）
    async fn atomic_write(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        let temp_path = self.get_temp_path(path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建父目录失败: {}", parent.display()))?;
        }

        // 写入临时文件
        tokio_fs::write(&temp_path, content)
            .await
            .with_context(|| format!("写入临时文件失败: {}", temp_path.display()))?;

        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            tokio_fs::set_permissions(&temp_path, perms)
                .await
                .map_err(|e| anyhow!("设置文件权限失败: {}", e))?;
        }

        if self.options.backup_enabled && tokio_fs::try_exists(path).await.unwrap_or(false) {
            self.create_backup(path).await?;
        }

        // 原子移动
        tokio_fs::rename(&temp_path, path).await.map_err(|e| {
            // 清理临时文件
            let _ = std::fs::remove_file(&temp_path);
            anyhow!("原子移动失败: {}", e)
        })?;

        Ok(())
    }

    /// 直接写入（同步）
    fn direct_write_sync(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| anyhow!("创建父目录失败: {}", e))?;
        }

        if self.options.backup_enabled && path.exists() {
            self.create_backup_sync(path)?;
        }

        // 直接写入
        fs::write(path, content).with_context(|| format!("写入文件失败: {}", path.display()))?;

        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            fs::set_permissions(path, perms).map_err(|e| anyhow!("设置文件权限失败: {}", e))?;
        }

        Ok(())
    }

    /// 直接写入（异步）
    async fn direct_write(&self, path: &Path, content: &[u8]) -> AppResult<()> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("创建父目录失败: {}", e))?;
        }

        if self.options.backup_enabled && tokio_fs::try_exists(path).await.unwrap_or(false) {
            self.create_backup(path).await?;
        }

        // 直接写入
        tokio_fs::write(path, content)
            .await
            .with_context(|| format!("写入文件失败: {}", path.display()))?;

        #[cfg(unix)]
        if let Some(permissions) = self.options.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(permissions);
            tokio_fs::set_permissions(path, perms)
                .await
                .map_err(|e| anyhow!("设置文件权限失败: {}", e))?;
        }

        Ok(())
    }

    /// 创建备份（同步）
    fn create_backup_sync(&self, path: &Path) -> AppResult<PathBuf> {
        let backup_path = self.get_backup_path(path);

        fs::copy(path, &backup_path).with_context(|| {
            format!(
                "创建备份失败: {} -> {}",
                path.display(),
                backup_path.display()
            )
        })?;

        // 清理旧备份
        self.cleanup_old_backups_sync(path)?;

        // 备份创建成功
        Ok(backup_path)
    }

    /// 创建备份（异步）
    async fn create_backup(&self, path: &Path) -> AppResult<PathBuf> {
        let backup_path = self.get_backup_path(path);

        tokio_fs::copy(path, &backup_path).await.with_context(|| {
            format!(
                "创建备份失败: {} -> {}",
                path.display(),
                backup_path.display()
            )
        })?;

        // 清理旧备份
        self.cleanup_old_backups(path).await?;

        // 备份创建成功
        Ok(backup_path)
    }

    /// 清理旧备份（同步）
    fn cleanup_old_backups_sync(&self, original_path: &Path) -> AppResult<()> {
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
            if let Err(_e) = fs::remove_file(path) {
                // 删除旧备份失败，继续处理其他文件
            } else {
                // 删除旧备份成功
            }
        }

        Ok(())
    }

    /// 清理旧备份（异步）
    async fn cleanup_old_backups(&self, original_path: &Path) -> AppResult<()> {
        let backup_prefix = self.get_backup_prefix(original_path);
        let mut backups = Vec::new();

        // 收集所有备份文件
        if let Ok(mut entries) = tokio_fs::read_dir(&self.paths.backups_dir).await {
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
            if let Err(_e) = tokio_fs::remove_file(path).await {
                // 删除旧备份失败，继续处理其他文件
            } else {
                // 删除旧备份成功
            }
        }

        Ok(())
    }

    /// 检查文件是否存在
    pub async fn exists(&self, path: &Path) -> bool {
        tokio_fs::try_exists(path).await.unwrap_or(false)
    }

    /// 检查文件是否存在（同步）
    pub fn exists_sync(&self, path: &Path) -> bool {
        path.exists()
    }

    /// 删除文件
    pub async fn remove_file(&self, path: &Path) -> AppResult<()> {
        if self.exists(path).await {
            tokio_fs::remove_file(path)
                .await
                .map_err(|e| anyhow!("删除文件失败: {}", e))?;
            // 文件删除成功
        }
        Ok(())
    }

    /// 删除文件（同步）
    pub fn remove_file_sync(&self, path: &Path) -> AppResult<()> {
        if self.exists_sync(path) {
            fs::remove_file(path).map_err(|e| anyhow!("删除文件失败: {}", e))?;
            // 文件删除成功
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
