use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("OrbitX app directory unavailable")]
    AppDirUnavailable,

    #[error("Failed to create directory: {path}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read file: {path}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file: {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse json: {path}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to serialize json")]
    SerializeJson(#[source] serde_json::Error),
}

pub type SettingsResult<T> = Result<T, SettingsError>;
