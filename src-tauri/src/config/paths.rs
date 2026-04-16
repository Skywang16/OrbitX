/*!
 * Configuration System Path Management Module
 *
 * Provides unified configuration file path management, supports cross-platform path resolution and directory creation.
 */

use crate::config::error::{ConfigPathsError, ConfigPathsResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration path manager
///
/// Responsible for managing all configuration-related file and directory paths, provides cross-platform path resolution functionality.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// Application data directory
    app_data_dir: PathBuf,

    /// Cached canonical application directory path (for path validation, avoid repeated syscalls)
    canonical_app_dir: PathBuf,

    /// Themes directory
    themes_dir: PathBuf,

    /// Backups directory
    backups_dir: PathBuf,

    /// Cache directory
    cache_dir: PathBuf,

    /// Logs directory
    logs_dir: PathBuf,

    /// Shell integration scripts directory
    shell_dir: PathBuf,

    /// Global Skills directory
    skills_dir: PathBuf,
}

impl ConfigPaths {
    /// Create a new configuration path manager
    ///
    /// Automatically determines configuration directory location based on current platform.
    ///
    /// # Errors
    ///
    /// Returns an error if user directory cannot be determined or necessary directories cannot be created.
    pub fn new() -> ConfigPathsResult<Self> {
        let app_data_dir = Self::get_app_data_dir()?;
        Self::with_app_data_dir(app_data_dir)
    }

    /// Create configuration path manager with custom application data directory
    ///
    /// # Parameters
    ///
    /// * `app_data_dir` - Custom application data directory path
    ///
    /// # Errors
    ///
    /// Returns an error if necessary directories cannot be created.
    pub fn with_app_data_dir<P: AsRef<Path>>(app_data_dir: P) -> ConfigPathsResult<Self> {
        let app_data_dir = app_data_dir.as_ref().to_path_buf();

        // Calculate and cache canonical path once
        let canonical_app_dir = match fs::canonicalize(&app_data_dir) {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!(
                    "Failed to canonicalize app data dir '{}': {}",
                    app_data_dir.display(),
                    err
                );
                app_data_dir.clone()
            }
        };

        let themes_dir = app_data_dir.join(crate::config::THEMES_DIR_NAME);
        let backups_dir = app_data_dir.join(crate::config::BACKUPS_DIR_NAME);
        let cache_dir = app_data_dir.join(crate::config::CACHE_DIR_NAME);
        let logs_dir = app_data_dir.join(crate::config::LOGS_DIR_NAME);
        let shell_dir = app_data_dir.join("shell");
        let skills_dir = app_data_dir.join("skills");

        let paths = Self {
            app_data_dir,
            canonical_app_dir,
            themes_dir,
            backups_dir,
            cache_dir,
            logs_dir,
            shell_dir,
            skills_dir,
        };

        // Ensure all necessary directories exist
        paths.ensure_directories_exist()?;

        Ok(paths)
    }

    /// Get application data directory
    ///
    /// Returns appropriate application data directory based on platform:
    /// - Windows: `%APPDATA%\OrbitX`
    /// - macOS: `~/Library/Application Support/OrbitX`
    /// - Linux: `~/.config/orbitx`
    fn get_app_data_dir() -> ConfigPathsResult<PathBuf> {
        let app_name = "OrbitX";

        #[cfg(target_os = "windows")]
        {
            use std::env;
            let appdata =
                env::var("APPDATA").map_err(|_| ConfigPathsError::ConfigDirectoryUnavailable)?;
            Ok(PathBuf::from(appdata).join(app_name))
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or(ConfigPathsError::HomeDirectoryUnavailable)?;
            Ok(home
                .join("Library")
                .join("Application Support")
                .join(app_name))
        }

        #[cfg(target_os = "linux")]
        {
            let config_dir =
                dirs::config_dir().ok_or(ConfigPathsError::ConfigDirectoryUnavailable)?;
            Ok(config_dir.join(app_name.to_lowercase()))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            let home = dirs::home_dir().ok_or(ConfigPathsError::HomeDirectoryUnavailable)?;
            Ok(home.join(".orbitx"))
        }
    }

    /// Get project themes directory
    ///
    /// Returns the config/themes directory path under project root
    /// Ensures all necessary directories exist
    fn ensure_directories_exist(&self) -> ConfigPathsResult<()> {
        let directories = [
            &self.app_data_dir,
            &self.themes_dir,
            &self.backups_dir,
            &self.cache_dir,
            &self.logs_dir,
            &self.shell_dir,
            &self.skills_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| ConfigPathsError::directory_create(dir.to_path_buf(), e))?;
            }
        }

        Ok(())
    }

    // Path getter methods

    /// Get application data directory path
    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    /// Get themes directory path
    pub fn themes_dir(&self) -> &Path {
        &self.themes_dir
    }

    /// Get specified theme file path
    pub fn theme_file<S: AsRef<str>>(&self, theme_name: S) -> PathBuf {
        self.themes_dir
            .join(format!("{}.json", theme_name.as_ref()))
    }

    /// Get backups directory path
    pub fn backups_dir(&self) -> &Path {
        &self.backups_dir
    }

    /// Get cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get logs directory path
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Get shell integration scripts directory path
    pub fn shell_dir(&self) -> &Path {
        &self.shell_dir
    }

    /// Get global Skills directory path
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    // Path validation and operation methods

    /// Validate if path is within allowed directory scope
    ///
    /// # Parameters
    ///
    /// * `path` - Path to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if path is safe, otherwise returns an error.
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> ConfigPathsResult<()> {
        let path = path.as_ref();
        let canonical_path = fs::canonicalize(path)
            .map_err(|e| ConfigPathsError::directory_access(path.to_path_buf(), e))?;

        // Use cached canonical path to save one syscall
        if !canonical_path.starts_with(&self.canonical_app_dir) {
            return Err(ConfigPathsError::validation(format!(
                "Path is not within allowed directory scope: {}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Get file size
    pub fn file_size<P: AsRef<Path>>(&self, path: P) -> ConfigPathsResult<u64> {
        let metadata = fs::metadata(path.as_ref())
            .map_err(|e| ConfigPathsError::directory_access(path.as_ref().to_path_buf(), e))?;

        Ok(metadata.len())
    }

    /// Delete file
    pub fn remove_file<P: AsRef<Path>>(&self, path: P) -> ConfigPathsResult<()> {
        let path = path.as_ref();

        // Validate path security
        self.validate_path(path)?;

        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| ConfigPathsError::directory_access(path.to_path_buf(), e))?;
        }

        Ok(())
    }

    /// Get directory size
    pub fn dir_size<P: AsRef<Path>>(&self, path: P) -> ConfigPathsResult<u64> {
        let path = path.as_ref();
        let mut total_size = 0;

        if path.is_dir() {
            let entries = fs::read_dir(path)
                .map_err(|e| ConfigPathsError::directory_access(path.to_path_buf(), e))?;

            for entry in entries {
                let entry =
                    entry.map_err(|e| ConfigPathsError::directory_access(path.to_path_buf(), e))?;

                let entry_path = entry.path();
                if entry_path.is_file() {
                    total_size += self.file_size(&entry_path)?;
                } else if entry_path.is_dir() {
                    total_size += self.dir_size(&entry_path)?;
                }
            }
        }

        Ok(total_size)
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self::new().expect("Failed to create default configuration paths")
    }
}

/// Convenience function to get global Skills directory path
///
/// Returns `~/.config/orbitx/skills` (or platform-specific path)
pub fn skills_dir() -> PathBuf {
    ConfigPaths::default().skills_dir().to_path_buf()
}
