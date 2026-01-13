use crate::config::defaults::create_default_config;
use crate::config::error::{ConfigError, ConfigResult};
use crate::config::types::AppConfig;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug)]
pub struct ConfigManager {
    path: PathBuf,
    cache: RwLock<AppConfig>,
}

impl ConfigManager {
    pub async fn new() -> ConfigResult<Self> {
        let app_dir = resolve_app_dir()?;
        fs::create_dir_all(&app_dir)
            .await
            .map_err(|e| ConfigError::Internal(format!("failed to create app dir: {e}")))?;

        let path = app_dir.join(CONFIG_FILE_NAME);

        let initial = if path.exists() {
            read_json(&path)
                .await
                .unwrap_or_else(|_| create_default_config())
        } else {
            create_default_config()
        };

        let manager = Self {
            path,
            cache: RwLock::new(initial),
        };

        // Always persist what we actually use (no migration; just overwrite).
        manager.config_save().await?;
        Ok(manager)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn config_get(&self) -> ConfigResult<AppConfig> {
        Ok(self.cache.read().await.clone())
    }

    pub async fn config_update<F>(&self, updater: F) -> ConfigResult<()>
    where
        F: FnOnce(&mut AppConfig) -> ConfigResult<()>,
    {
        {
            let mut guard = self.cache.write().await;
            updater(&mut guard)?;
        }
        self.config_save().await
    }

    pub async fn config_set(&self, new_config: AppConfig) -> ConfigResult<()> {
        {
            let mut guard = self.cache.write().await;
            *guard = new_config;
        }
        self.config_save().await
    }

    pub async fn config_save(&self) -> ConfigResult<()> {
        let config = self.cache.read().await.clone();
        write_json(&self.path, &config).await
    }
}

fn resolve_app_dir() -> ConfigResult<PathBuf> {
    if let Ok(dir) = std::env::var("OrbitX_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let Some(data_dir) = dirs::data_dir() else {
        return Err(ConfigError::Internal("system data_dir unavailable".into()));
    };
    Ok(data_dir.join("OrbitX"))
}

async fn read_json(path: &Path) -> ConfigResult<AppConfig> {
    let raw = fs::read_to_string(path).await.map_err(|e| {
        ConfigError::Internal(format!(
            "failed to read config file {}: {e}",
            path.display()
        ))
    })?;
    Ok(serde_json::from_str(&raw)?)
}

async fn write_json(path: &Path, config: &AppConfig) -> ConfigResult<()> {
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, format!("{json}\n")).await.map_err(|e| {
        ConfigError::Internal(format!(
            "failed to write config file {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}
