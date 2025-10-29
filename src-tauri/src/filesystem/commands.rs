use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use ignore::WalkBuilder;
use std::path::PathBuf;

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
