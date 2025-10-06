use std::path::{Component, Path, PathBuf};

use crate::agent::tools::error::ToolExecutorError;

/// List of extensions treated as binary to mirror front-end safeguards.
fn binary_extension(ext: &str) -> bool {
    matches!(
        ext,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "bmp"
            | "tiff"
            | "tif"
            | "webp"
            | "ico"
            | "svg"
            | "mp3"
            | "wav"
            | "flac"
            | "aac"
            | "ogg"
            | "m4a"
            | "wma"
            | "mp4"
            | "avi"
            | "mkv"
            | "mov"
            | "wmv"
            | "flv"
            | "webm"
            | "3gp"
            | "zip"
            | "rar"
            | "7z"
            | "tar"
            | "gz"
            | "bz2"
            | "xz"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "app"
            | "deb"
            | "rpm"
            | "dmg"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "pdf"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "eot"
            | "db"
            | "sqlite"
            | "sqlite3"
            | "class"
            | "jar"
            | "war"
            | "ear"
            | "pyc"
            | "pyo"
            | "o"
            | "obj"
            | "bin"
            | "dat"
            | "iso"
            | "img"
    )
}

/// Returns true when the provided path likely points to a binary file.
pub fn is_probably_binary(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| binary_extension(&ext.to_ascii_lowercase()))
        .unwrap_or(false)
}

/// Normalize path components without touching the filesystem.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// Resolve a potentially relative path against the provided cwd and normalize it.
pub fn ensure_absolute(path: &str, cwd: &str) -> Result<PathBuf, ToolExecutorError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(ToolExecutorError::InvalidArguments {
            tool_name: "file_utils".to_string(),
            error: "Path cannot be empty".to_string(),
        });
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        return Ok(normalize_path(&candidate));
    }

    let cwd_path = PathBuf::from(cwd);
    if !cwd_path.is_absolute() {
        return Err(ToolExecutorError::InvalidArguments {
            tool_name: "file_utils".to_string(),
            error: "Working directory is invalid; cannot resolve relative path".to_string(),
        });
    }

    Ok(normalize_path(&cwd_path.join(candidate)))
}
