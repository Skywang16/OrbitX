use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use ignore::gitignore::GitignoreBuilder;
use ignore::WalkBuilder;
use std::path::PathBuf;

/// 扩展的目录条目，包含 gitignore 状态
#[derive(serde::Serialize)]
pub struct DirEntryExt {
    pub name: String,
    pub is_directory: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_ignored: bool,
}

/// 读取目录内容（完整版，包含 gitignore 状态）
#[tauri::command]
pub async fn fs_read_dir(path: String) -> TauriApiResult<Vec<DirEntryExt>> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(api_error!("common.not_found"));
    }
    if !root.is_dir() {
        return Ok(api_error!("common.invalid_path"));
    }

    // 手动创建 gitignore 检查器
    let mut gi_builder = GitignoreBuilder::new(&root);

    // 尝试手动添加当前目录的 .gitignore 文件
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        let _ = gi_builder.add(&gitignore_path);
    }

    // 向上查找并添加父目录的 .gitignore
    let mut parent = root.parent();
    while let Some(p) = parent {
        let parent_gitignore = p.join(".gitignore");
        if parent_gitignore.exists() {
            let _ = gi_builder.add(&parent_gitignore);
        }
        // 检查是否到达了 git 仓库根目录或文件系统根目录
        if p.join(".git").exists() || p.parent().is_none() {
            break;
        }
        parent = p.parent();
    }

    let gitignore = match gi_builder.build() {
        Ok(g) => g,
        Err(_) => GitignoreBuilder::new(&root).build().unwrap(),
    };

    let mut entries = Vec::new();

    // 读取目录内容
    let read_dir = match std::fs::read_dir(&root) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to read directory: {}", e);
            return Ok(api_success!(entries));
        }
    };

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let file_path = entry.path();
        let file_name = entry.file_name();

        let name = file_name.to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

        // 检查是否被 gitignore 忽略（相对于根目录）
        let relative_path = file_path.strip_prefix(&root).unwrap_or(&file_path);
        let is_ignored = gitignore.matched(relative_path, is_dir).is_ignore();

        entries.push(DirEntryExt {
            name,
            is_directory: is_dir,
            is_file,
            is_symlink,
            is_ignored,
        });
    }

    Ok(api_success!(entries))
}

/// 内置忽略目录 - 递归遍历时自动跳过这些大目录
/// 注意：如果用户直接指定这些目录作为根路径，仍然可以访问
const BUILTIN_SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".turbo",
    "__pycache__",
    ".pytest_cache",
    "venv",
    ".venv",
    "vendor",
    "coverage",
    ".nyc_output",
    "bower_components",
];

pub(crate) async fn fs_list_directory(
    path: String,
    recursive: bool,
) -> TauriApiResult<Vec<String>> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(api_error!("common.not_found"));
    }
    if !root.is_dir() {
        return Ok(api_error!("common.invalid_path"));
    }

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .standard_filters(true)
        .sort_by_file_name(|a, b| a.cmp(b));

    if !recursive {
        builder.max_depth(Some(1));
    }

    // 递归时过滤内置忽略目录（depth > 0 才过滤，这样用户直接指定这些目录仍可访问）
    if recursive {
        builder.filter_entry(|entry| {
            // depth 0 是根目录本身，不过滤
            if entry.depth() == 0 {
                return true;
            }
            // 只过滤目录
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if is_dir {
                let name = entry.file_name().to_string_lossy();
                if BUILTIN_SKIP_DIRS.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        });
    }

    let mut entries: Vec<(String, bool)> = Vec::new();

    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let p = entry.path();
        let rel = p.strip_prefix(&root).unwrap_or(p);
        let is_dir = entry
            .file_type()
            .map(|ft| ft.is_dir())
            .unwrap_or_else(|| p.is_dir());
        let mut name = rel.to_string_lossy().to_string();
        if is_dir && !name.ends_with('/') {
            name.push('/');
        }
        entries.push((name, is_dir));
    }

    // 排序：目录在前，字典序
    entries.sort_unstable_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.cmp(&b.0),
    });

    let out: Vec<String> = entries.into_iter().map(|(s, _)| s).collect();
    Ok(api_success!(out))
}
