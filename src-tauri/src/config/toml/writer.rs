//! TOML配置写入器

use crate::config::error::{TomlConfigError, TomlConfigResult};
use crate::config::types::AppConfig;
use std::{io, path::PathBuf};
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
    pub async fn config_save(&self, config: &AppConfig) -> TomlConfigResult<()> {
        // 确保配置目录存在
        self.ensure_config_directory().await?;

        // 序列化配置为TOML
        let toml_content = toml::to_string_pretty(config)?;

        self.write_config_with_retry(&toml_content, 3).await?;

        Ok(())
    }

    /// 确保配置目录存在
    async fn ensure_config_directory(&self) -> TomlConfigResult<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                TomlConfigError::Io(io::Error::new(
                    e.kind(),
                    format!("Failed to create config directory: {} - {}", parent.display(), e),
                ))
            })?;
        }
        Ok(())
    }

    /// 带重试机制的配置文件写入
    async fn write_config_with_retry(&self, content: &str, max_retries: usize) -> TomlConfigResult<()> {
        let mut last_error: Option<TomlConfigError> = None;

        for attempt in 1..=max_retries {
            match self.atomic_write_config(content).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("配置文件在第{}次尝试后写入成功", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    warn!("配置文件写入失败 (尝试 {}/{}) : {}", attempt, max_retries, e);
                    last_error = Some(e);

                    if attempt < max_retries {
                        // 短暂等待后重试
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| TomlConfigError::Internal("Config file write failed, unknown error".to_string())))
    }

    /// 原子写入配置：优先直接写入，失败则采用临时文件 + 重命名策略
    async fn atomic_write_config(&self, content: &str) -> TomlConfigResult<()> {
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

        debug!("尝试直接写入配置文件");

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
                warn!("直接写入失败，尝试原子重命名: {}", write_err);

                let temp_path = self
                    .config_path
                    .with_extension(&format!("tmp.{}", std::process::id()));

                if temp_path.exists() {
                    let _ = fs::remove_file(&temp_path).await;
                }

                // 写入临时文件
                fs::write(&temp_path, content).await.map_err(|e| {
                    TomlConfigError::Io(io::Error::new(e.kind(), format!(
                        "Failed to write temp config file: {}",
                        temp_path.display()
                    )))
                })?;

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

                        if let Some(backup) = backup_path {
                            if let Err(restore_err) = fs::copy(&backup, &self.config_path).await {
                                warn!("恢复配置备份失败: {}", restore_err);
                            } else {
                                info!("已恢复配置备份");
                            }
                            let _ = fs::remove_file(&backup).await;
                        }

                        Err(TomlConfigError::Internal(format!(
                            "Config file write failed - 直接写入错误: {}, 原子写入错误: {}",
                            write_err, rename_err
                        )))
                    }
                }
            }
        }
    }

    /// 创建备份并恢复默认配置
    pub async fn create_backup_and_use_default(&self) -> TomlConfigResult<AppConfig> {
        use crate::config::defaults::create_default_config;

        if self.config_path.exists() {
            let backup_path = self.config_path.with_extension("backup");
            if let Err(e) = fs::copy(&self.config_path, &backup_path).await {
                warn!("Failed to create config backup: {}", e);
            } else {
                info!("Created config backup: {:?}", backup_path);
            }
        }

        // 使用默认配置
        let default_config = create_default_config();
        self.config_save(&default_config).await?;

        Ok(default_config)
    }
}
