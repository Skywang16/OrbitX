use crate::settings::error::{SettingsError, SettingsResult};
use crate::settings::types::{EffectiveSettings, PermissionRules, Settings};
use std::path::{Path, PathBuf};
use tokio::fs;

const GLOBAL_SETTINGS_FILE_NAME: &str = "settings.json";
const WORKSPACE_SETTINGS_REL_PATH: &str = ".orbitx/settings.json";
const SETTINGS_SCHEMA_URL: &str = "https://orbitx.app/schemas/settings.json";

#[derive(Debug, Clone)]
pub struct SettingsManager {
    app_dir: PathBuf,
    global_settings_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> SettingsResult<Self> {
        let app_dir = Self::resolve_app_dir()?;
        Self::with_app_dir(app_dir)
    }

    pub fn with_app_dir(app_dir: PathBuf) -> SettingsResult<Self> {
        std::fs::create_dir_all(&app_dir).map_err(|source| SettingsError::CreateDir {
            path: app_dir.clone(),
            source,
        })?;

        let global_settings_path = app_dir.join(GLOBAL_SETTINGS_FILE_NAME);

        Ok(Self {
            app_dir,
            global_settings_path,
        })
    }

    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn global_settings_path(&self) -> &Path {
        &self.global_settings_path
    }

    pub fn workspace_settings_path(workspace_root: impl AsRef<Path>) -> PathBuf {
        workspace_root.as_ref().join(WORKSPACE_SETTINGS_REL_PATH)
    }

    pub async fn get_global_settings(&self) -> SettingsResult<Settings> {
        if !self.global_settings_path.exists() {
            let settings = Self::default_global_settings();
            self.update_global_settings(&settings).await?;
            return Ok(settings);
        }

        self.read_json(&self.global_settings_path).await
    }

    pub async fn update_global_settings(&self, settings: &Settings) -> SettingsResult<()> {
        self.write_json(&self.global_settings_path, settings).await
    }

    pub async fn get_workspace_settings(
        &self,
        workspace_root: impl AsRef<Path>,
    ) -> SettingsResult<Option<Settings>> {
        let path = Self::workspace_settings_path(workspace_root);
        if !path.exists() {
            return Ok(None);
        }
        self.read_json(&path).await.map(Some)
    }

    pub async fn update_workspace_settings(
        &self,
        workspace_root: impl AsRef<Path>,
        settings: &Settings,
    ) -> SettingsResult<()> {
        let path = Self::workspace_settings_path(workspace_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|source| SettingsError::CreateDir {
                    path: parent.to_path_buf(),
                    source,
                })?;
        }
        self.write_json(&path, settings).await
    }

    pub async fn get_effective_settings(
        &self,
        workspace_root: Option<PathBuf>,
    ) -> SettingsResult<EffectiveSettings> {
        let global = self.get_global_settings().await?;
        let workspace = match workspace_root {
            Some(root) => self.get_workspace_settings(root).await?,
            None => None,
        };

        Ok(EffectiveSettings::merge(&global, workspace.as_ref()))
    }

    async fn read_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> SettingsResult<T> {
        let raw = fs::read_to_string(path)
            .await
            .map_err(|source| SettingsError::ReadFile {
                path: path.to_path_buf(),
                source,
            })?;

        serde_json::from_str::<T>(&raw).map_err(|source| SettingsError::ParseJson {
            path: path.to_path_buf(),
            source,
        })
    }

    async fn write_json<T: serde::Serialize>(&self, path: &Path, data: &T) -> SettingsResult<()> {
        let json = serde_json::to_string_pretty(data).map_err(SettingsError::SerializeJson)?;
        fs::write(path, format!("{json}\n"))
            .await
            .map_err(|source| SettingsError::WriteFile {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(())
    }

    fn default_global_settings() -> Settings {
        Settings {
            schema: Some(SETTINGS_SCHEMA_URL.to_string()),
            permissions: PermissionRules {
                allow: vec![
                    "Read(${workspaceFolder}/**)".into(),
                    "List(${workspaceFolder}/**)".into(),
                    "Grep".into(),
                    "Semantic_Search".into(),
                    "Syntax_Diagnostics".into(),
                    "Terminal".into(),
                    "Task".into(),
                    "Todowrite".into(),
                    "Bash(ls:*)".into(),
                    "Bash(cat:${workspaceFolder}/**)".into(),
                    "Bash(head:${workspaceFolder}/**)".into(),
                    "Bash(tail:${workspaceFolder}/**)".into(),
                    "Bash(git status:*)".into(),
                    "Bash(git diff:*)".into(),
                    "Bash(git log:*)".into(),
                    "Bash(git branch:*)".into(),
                ],
                deny: vec![
                    "Bash(rm -rf /)".into(),
                    "Bash(sudo:*)".into(),
                    "Bash(git push --force:*)".into(),
                    "Write(/etc/**)".into(),
                    "Write(/usr/**)".into(),
                    "Write(/System/**)".into(),
                    "Write(~/.ssh/**)".into(),
                ],
                ask: vec![
                    "external_directory".into(),
                    "Write(**)".into(),
                    "Edit(**)".into(),
                    "Bash(rm:*)".into(),
                    "Bash(mv:*)".into(),
                    "Bash(git push:*)".into(),
                    "Bash(git commit:*)".into(),
                    "Bash(git checkout:*)".into(),
                    "Bash(git reset:*)".into(),
                    "WebFetch(*)".into(),
                ],
            },
            ..Default::default()
        }
    }

    fn resolve_app_dir() -> SettingsResult<PathBuf> {
        if let Ok(dir) = std::env::var("OrbitX_DATA_DIR") {
            return Ok(PathBuf::from(dir));
        }

        let Some(data_dir) = dirs::data_dir() else {
            return Err(SettingsError::AppDirUnavailable);
        };

        Ok(data_dir.join("OrbitX"))
    }
}
