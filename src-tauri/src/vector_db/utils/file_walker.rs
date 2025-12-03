use ignore::{DirEntry, WalkBuilder};
use std::path::{Path, PathBuf};

pub fn collect_source_files(root: &Path, max_size: u64) -> Vec<PathBuf> {
  let mut files = Vec::new();
  let mut builder = WalkBuilder::new(root);
  builder
    .hidden(true)
    .ignore(true)
    .git_ignore(true)
    .git_exclude(true)
    .parents(true)
    .max_depth(None)
    .filter_entry(|e| filter_dirs(e));

  for result in builder.build() {
    if let Ok(entry) = result {
      let path = entry.path();
      if path.is_file() {
        if let Ok(meta) = std::fs::metadata(path) {
          if meta.len() <= max_size {
            files.push(path.to_path_buf());
          }
        }
      }
    }
  }
  files
}

fn filter_dirs(e: &DirEntry) -> bool {
  let path = e.path();
  if !path.is_dir() {
    return true;
  }
  if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
    // Common excludes
    const EXCLUDES: &[&str] = &[
      ".git",
      ".svn",
      "node_modules",
      "target",
      "dist",
      "build",
      ".idea",
      ".vscode",
      ".DS_Store",
    ];
    return !EXCLUDES.iter().any(|d| *d == name);
  }
  true
}
