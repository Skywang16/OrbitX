/*!
 * TOML配置写入器
 *
 * 负责将配置写入文件系统，支持原子写入和备份恢复
 */

use crate::{config::types::AppConfig, utils::error::AppResult};
use anyhow::{anyhow, Context};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

/// TOML配置写入器
pub struct TomlConfigWriter {
    config_path: PathBuf,
}

impl TomlConfigWriter {
    /// 创建新的配置写入器
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// 保存配置到文件
    pub async fn config_save(&self, config: &AppConfig) -> AppResult<()> {
        // 确保配置目录存在
        self.ensure_config_directory().await?;

        // 序列化配置为TOML
        let toml_content = toml::to_string_pretty(config).context("配置序列化为TOML失败")?;

        // 带重试的配置文件写入
        self.write_config_with_retry(&toml_content, 3).await?;

        Ok(())
    }

    /// 带重试机制的配置文件写入
    async fn write_config_with_retry(&self, content: &str, max_retries: usize) -> AppResult<()> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match self.atomic_write_config(content).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("配置文件在第{}次尝试后写入成功", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    warn!("配置文件写入失败 (尝试 {}/{}): {}", attempt, max_retries, e);
                    last_error = Some(e);

                    if attempt < max_retries {
                        // 短暂等待后重试
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempt as u64))
                            .await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("配置文件写入失败，未知错误")))
    }

    /// 确保配置目录存在
    async fn ensure_config_directory(&self) -> AppResult<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;

            // 验证目录权限
            self.verify_directory_permissions(parent).await?;
        }
        Ok(())
    }

    /// 验证目录权限
    async fn verify_directory_permissions(&self, dir: &std::path::Path) -> AppResult<()> {
        // 尝试在目录中创建一个测试文件来验证写权限
        let test_file = dir.join(&format!(".orbitx_test_{}", std::process::id()));

        match fs::write(&test_file, "test").await {
            Ok(()) => {
                // 成功创建测试文件，立即删除
                let _ = fs::remove_file(&test_file).await;
                Ok(())
            }
            Err(e) => Err(anyhow!("配置目录权限不足: {} - {}", dir.display(), e)),
        }
    }

    /// 原子写入配置文件
    async fn atomic_write_config(&self, content: &str) -> AppResult<()> {
        // 简化的配置文件写入策略
        // 在 macOS 上，原子重命名有时会失败，我们使用更直接的方法

        // 首先尝试直接写入（最可靠的方法）
        debug!("尝试直接写入配置文件");

        // 创建备份（如果原文件存在）
        let backup_path = if self.config_path.exists() {
            let backup = self.config_path.with_extension("backup");
            match fs::copy(&self.config_path, &backup).await {
                Ok(_) => {
                    debug!("已创建配置备份: {}", backup.display());
                    Some(backup)
                }
                Err(e) => {
                    warn!("创建配置备份失败: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 直接写入配置文件
        match fs::write(&self.config_path, content).await {
            Ok(()) => {
                debug!("配置文件写入成功");

                // 删除备份文件（如果存在）
                if let Some(backup) = backup_path {
                    let _ = fs::remove_file(&backup).await;
                }

                Ok(())
            }
            Err(write_err) => {
                warn!("直接写入失败，尝试原子写入方法: {}", write_err);

                // 如果直接写入失败，尝试原子写入方法
                let temp_path = self
                    .config_path
                    .with_extension(&format!("tmp.{}", std::process::id()));

                // 清理可能存在的临时文件
                if temp_path.exists() {
                    let _ = fs::remove_file(&temp_path).await;
                }

                // 写入临时文件
                fs::write(&temp_path, content)
                    .await
                    .with_context(|| format!("无法写入临时配置文件: {}", temp_path.display()))?;

                // 尝试重命名
                match fs::rename(&temp_path, &self.config_path).await {
                    Ok(()) => {
                        debug!("原子写入成功");

                        // 删除备份文件（如果存在）
                        if let Some(backup) = backup_path {
                            let _ = fs::remove_file(&backup).await;
                        }

                        Ok(())
                    }
                    Err(rename_err) => {
                        // 清理临时文件
                        let _ = fs::remove_file(&temp_path).await;

                        // 如果有备份，尝试恢复
                        if let Some(backup) = backup_path {
                            if let Err(restore_err) = fs::copy(&backup, &self.config_path).await {
                                warn!("恢复配置备份失败: {}", restore_err);
                            } else {
                                info!("已恢复配置备份");
                            }
                            let _ = fs::remove_file(&backup).await;
                        }

                        Err(anyhow!(
                            "配置文件写入失败 - 直接写入错误: {}, 原子写入错误: {}",
                            write_err,
                            rename_err
                        ))
                    }
                }
            }
        }
    }

    /// 创建备份并恢复默认配置
    pub async fn create_backup_and_use_default(&self) -> AppResult<AppConfig> {
        use crate::config::defaults::create_default_config;

        // 创建备份
        if self.config_path.exists() {
            let backup_path = self.config_path.with_extension("backup");
            if let Err(e) = fs::copy(&self.config_path, &backup_path).await {
                warn!("创建配置备份失败: {}", e);
            } else {
                info!("已创建配置备份: {:?}", backup_path);
            }
        }

        // 使用默认配置
        let default_config = create_default_config();
        self.config_save(&default_config).await?;

        Ok(default_config)
    }
}
