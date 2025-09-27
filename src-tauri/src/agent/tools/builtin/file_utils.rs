use std::path::Path;

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

/// Simple helper mirroring front-end absolute-path guard.
pub fn ensure_absolute(path: &Path) -> Result<(), String> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err("Path must be an absolute path".to_string())
    }
}
