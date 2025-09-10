/*!
 * TOML配置读取器
 *
 * 负责从文件系统读取和解析TOML配置文件
 */

use crate::{
    config::{defaults::create_default_config, paths::ConfigPaths, types::AppConfig},
    utils::error::AppResult,
};
use anyhow::{anyhow, Context};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

/// TOML配置读取器
pub struct TomlConfigReader {
    config_path: PathBuf,
    #[allow(dead_code)]
    paths: ConfigPaths,
}

impl TomlConfigReader {
    /// 创建新的配置读取器
    pub fn new() -> AppResult<Self> {
        let paths = ConfigPaths::new()?;
        let config_path = paths.config_file();

        Ok(Self { config_path, paths })
    }

    /// 创建指定配置路径的配置读取器（主要用于测试）
    #[cfg(test)]
    pub fn new_with_config_path(config_path: PathBuf) -> AppResult<Self> {
        // 为测试创建一个虚拟的 ConfigPaths
        let temp_dir = config_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("配置文件路径无效"))?;
        let paths = ConfigPaths::with_app_data_dir(temp_dir)?;

        Ok(Self { config_path, paths })
    }

    /// 从文件系统加载TOML配置
    /// 如果文件不存在则尝试从资源文件复制，最后创建默认配置
    pub async fn load_config(&self) -> AppResult<AppConfig> {
        debug!("开始加载TOML配置: {:?}", self.config_path);

        if self.config_path.exists() {
            // 读取现有配置文件
            let content = fs::read_to_string(&self.config_path)
                .await
                .with_context(|| format!("无法读取配置文件: {}", self.config_path.display()))?;

            // 解析TOML内容
            match self.parse_toml_content(&content) {
                Ok(parsed_config) => {
                    info!("配置文件解析成功");
                    Ok(parsed_config)
                }
                Err(e) => {
                    warn!("配置文件解析失败: {}, 使用默认配置", e);
                    // 返回错误，让调用者处理备份和默认配置
                    Err(e)
                }
            }
        } else {
            info!("配置文件不存在，尝试复制打包的配置文件");

            // 尝试从资源文件复制配置
            match self.copy_bundled_config().await {
                Ok(config) => {
                    info!("成功复制打包的配置文件");
                    Ok(config)
                }
                Err(_) => {
                    info!("未找到打包的配置文件，返回默认配置");
                    Ok(create_default_config())
                }
            }
        }
    }

    /// 解析TOML内容为配置结构
    pub fn parse_toml_content(&self, content: &str) -> AppResult<AppConfig> {
        toml::from_str::<AppConfig>(content)
            .with_context(|| format!("TOML配置解析失败 (文件: {})", self.config_path.display()))
    }

    /// 获取配置文件路径
    pub fn get_config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// 复制打包的配置文件
    async fn copy_bundled_config(&self) -> AppResult<AppConfig> {
        // 尝试从应用资源中获取配置文件
        let bundled_config_path = self.get_bundled_config_path()?;

        if bundled_config_path.exists() {
            // 复制文件到用户配置目录
            fs::copy(&bundled_config_path, &self.config_path)
                .await
                .with_context(|| "复制打包配置文件失败")?;

            // 读取并解析复制的配置文件
            let content = fs::read_to_string(&self.config_path)
                .await
                .with_context(|| "读取复制的配置文件失败")?;

            self.parse_toml_content(&content)
        } else {
            Err(anyhow!("未找到打包的配置文件"))
        }
    }

    /// 获取打包配置文件路径
    fn get_bundled_config_path(&self) -> AppResult<PathBuf> {
        // 在 Tauri 中，资源文件通常位于应用包中
        let exe_dir = std::env::current_exe()
            .with_context(|| "无法获取可执行文件路径")?
            .parent()
            .ok_or_else(|| anyhow!("无法获取可执行文件目录"))?
            .to_path_buf();

        // 在不同平台上，资源文件的位置可能不同
        #[cfg(target_os = "macos")]
        {
            // macOS: 资源文件在 .app/Contents/Resources/ 目录下
            let app_bundle = exe_dir
                .parent()
                .and_then(|p| p.parent())
                .ok_or_else(|| anyhow!("无法找到 macOS 应用包路径"))?;
            Ok(app_bundle.join("Resources").join("config.toml"))
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Windows/Linux: 资源文件可能在可执行文件同级目录
            Ok(exe_dir.join("config.toml"))
        }
    }
}
