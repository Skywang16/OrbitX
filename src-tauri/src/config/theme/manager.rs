use super::types::Theme;
use crate::config::error::{ThemeConfigError, ThemeConfigResult};
use crate::config::paths::ConfigPaths;
use crate::storage::cache::UnifiedCache;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc, time::SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileWrapper {
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeIndexEntry {
    pub name: String,
    pub file: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    pub builtin: bool,
    pub file_size: Option<u64>,
    pub last_modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Default)]
pub struct ThemeManagerOptions {
    pub auto_refresh_index: bool,
    pub index_refresh_interval: u64,
}

pub struct ThemeManager {
    paths: ConfigPaths,
    _cache: Arc<UnifiedCache>,
}

impl ThemeManager {
    pub async fn new(
        paths: ConfigPaths,
        _options: ThemeManagerOptions,
        cache: Arc<UnifiedCache>,
    ) -> ThemeConfigResult<Self> {
        Ok(Self {
            paths,
            _cache: cache,
        })
    }

    pub fn paths(&self) -> &ConfigPaths {
        &self.paths
    }

    pub async fn list_themes(&self) -> ThemeConfigResult<Vec<ThemeIndexEntry>> {
        let mut out = Vec::new();
        let themes_dir = self.paths.themes_dir();

        if !themes_dir.exists() {
            return Ok(out);
        }

        let entries = fs::read_dir(themes_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if file_name == "index.json" {
                continue;
            }

            let meta = fs::metadata(&path)?;
            let file_size = Some(meta.len());
            let last_modified = meta.modified().ok();

            let raw = fs::read_to_string(&path)?;
            let wrapper: ThemeFileWrapper = match serde_json::from_str(&raw) {
                Ok(w) => w,
                Err(_) => continue,
            };

            out.push(ThemeIndexEntry {
                name: wrapper.theme.name.clone(),
                file: file_name.to_string(),
                theme_type: wrapper.theme.theme_type.to_string(),
                builtin: true,
                file_size,
                last_modified,
            });
        }

        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub async fn load_theme(&self, theme_name: &str) -> ThemeConfigResult<Theme> {
        let theme_name = theme_name.trim();
        if theme_name.is_empty() {
            return Err(ThemeConfigError::Validation {
                reason: "theme_name cannot be empty".into(),
            });
        }

        let theme_path = self.paths.themes_dir().join(format!("{theme_name}.json"));
        if !theme_path.exists() {
            return Err(ThemeConfigError::NotFound {
                name: theme_name.to_string(),
            });
        }

        let raw = fs::read_to_string(&theme_path)?;
        let wrapper: ThemeFileWrapper = serde_json::from_str(&raw)?;

        let validation = ThemeValidator::validate_theme(&wrapper.theme);
        if !validation.is_valid {
            return Err(ThemeConfigError::Validation {
                reason: validation
                    .errors
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| "invalid theme".into()),
            });
        }

        Ok(wrapper.theme)
    }

    pub async fn save_theme(&self, theme: &Theme) -> ThemeConfigResult<PathBuf> {
        let validation = ThemeValidator::validate_theme(theme);
        if !validation.is_valid {
            return Err(ThemeConfigError::Validation {
                reason: validation
                    .errors
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| "invalid theme".into()),
            });
        }

        let name = theme.name.trim();
        if name.is_empty() {
            return Err(ThemeConfigError::Validation {
                reason: "theme.name cannot be empty".into(),
            });
        }

        let path = self.paths.themes_dir().join(format!("{name}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let wrapper = ThemeFileWrapper {
            theme: theme.clone(),
        };
        let json = serde_json::to_string_pretty(&wrapper)?;
        fs::write(&path, format!("{json}\n"))?;
        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

pub struct ThemeValidator;

impl ThemeValidator {
    pub fn validate_theme(theme: &Theme) -> ThemeValidationResult {
        let mut errors = Vec::new();

        if theme.name.trim().is_empty() {
            errors.push("theme.name cannot be empty".into());
        }

        fn ensure_color(label: &str, value: &str, errors: &mut Vec<String>) {
            if value.trim().is_empty() {
                errors.push(format!("{label} cannot be empty"));
            }
        }

        ensure_color("ansi.black", &theme.ansi.black, &mut errors);
        ensure_color("ansi.white", &theme.ansi.white, &mut errors);
        ensure_color("ui.bg_400", &theme.ui.bg_400, &mut errors);
        ensure_color("ui.text_100", &theme.ui.text_100, &mut errors);

        ThemeValidationResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }
}
