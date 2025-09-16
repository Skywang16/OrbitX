//! 文件系统补全提供者

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use anyhow::Context;
use async_trait::async_trait;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// 文件系统补全提供者
pub struct FilesystemProvider {
    /// 模糊匹配器
    matcher: SkimMatcherV2,
    /// 最大搜索深度
    max_depth: usize,
    /// 是否显示隐藏文件
    show_hidden: bool,
}

impl FilesystemProvider {
    /// 创建新的文件系统提供者
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            max_depth: 3,
            show_hidden: false,
        }
    }

    /// 设置最大搜索深度
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// 设置是否显示隐藏文件
    pub fn with_show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// 解析路径，处理相对路径和绝对路径
    fn resolve_path(&self, input: &str, working_dir: &Path) -> PathBuf {
        let path = Path::new(input);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_dir.join(path)
        }
    }

    /// 获取目录下的文件和子目录
    async fn get_directory_entries(&self, dir_path: &Path) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(items);
        }

        let mut entries = fs::read_dir(dir_path).await.context("读取目录失败")?;

        while let Some(entry) = entries.next_entry().await.context("读取目录项失败")? {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // 跳过隐藏文件（除非明确要求显示）
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().await.context("获取文件元数据失败")?;

            let completion_type = if metadata.is_dir() {
                CompletionType::Directory
            } else {
                CompletionType::File
            };

            let mut item = CompletionItem::new(file_name.clone(), completion_type)
                .with_source("filesystem".to_string());

            // 为目录添加斜杠
            if metadata.is_dir() {
                item = item.with_display_text(format!("{file_name}/"));
            }

            // 添加文件大小信息
            if metadata.is_file() {
                let size = metadata.len();
                let size_str = format_file_size(size);
                item = item.with_metadata("size".to_string(), size_str);
            }

            items.push(item);
        }

        Ok(items)
    }

    /// 递归搜索文件（用于深度搜索）
    fn search_files_recursive(
        &self,
        search_dir: &Path,
        pattern: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for entry in WalkDir::new(search_dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // 跳过隐藏文件
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            // 模糊匹配
            if let Some(score) = self.matcher.fuzzy_match(&file_name, pattern) {
                let completion_type = if path.is_dir() {
                    CompletionType::Directory
                } else {
                    CompletionType::File
                };

                let relative_path = path
                    .strip_prefix(search_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let mut item = CompletionItem::new(relative_path.clone(), completion_type)
                    .with_score(score as f64)
                    .with_source("filesystem".to_string());

                if path.is_dir() {
                    item = item.with_display_text(format!("{relative_path}/"));
                }

                items.push(item);
            }
        }

        // 按分数排序
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(items)
    }
}

#[async_trait]
impl CompletionProvider for FilesystemProvider {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // 只在以下情况提供文件系统补全：
        // 1. 当前词包含路径分隔符（明确的路径输入）
        if context.current_word.contains('/') || context.current_word.contains('\\') {
            return true;
        }

        // 2. 当前词以 . 开头（相对路径）
        if context.current_word.starts_with('.') {
            return true;
        }

        // 3. 当前词以 ~ 开头（用户主目录）
        if context.current_word.starts_with('~') {
            return true;
        }

        // 4. 当前词以 / 开头（绝对路径）
        if context.current_word.starts_with('/') {
            return true;
        }

        // 不再提供"空词"的文件补全，避免干扰命令选项补全
        false
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        let current_word = &context.current_word;

        if current_word.is_empty() {
            return self.get_directory_entries(&context.working_directory).await;
        }

        // 解析路径
        let full_path = self.resolve_path(current_word, &context.working_directory);

        let result = if full_path.is_dir() {
            self.get_directory_entries(&full_path).await
        } else {
            let parent_dir = full_path.parent().unwrap_or(&context.working_directory);

            let file_name = full_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if parent_dir.exists() && parent_dir.is_dir() {
                let mut items = self.get_directory_entries(parent_dir).await?;

                // 过滤匹配的项
                if !file_name.is_empty() {
                    items = items
                        .into_iter()
                        .filter_map(|mut item| {
                            if let Some(score) = self.matcher.fuzzy_match(&item.text, &file_name) {
                                // 更新分数
                                item.score = score as f64;
                                Some(item)
                            } else {
                                None
                            }
                        })
                        .collect();

                    // 按分数排序
                    items.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }

                Ok(items)
            } else {
                self.search_files_recursive(&context.working_directory, &file_name)
            }
        };

        result
    }

    fn priority(&self) -> i32 {
        10 // 文件系统补全优先级较高
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// 格式化文件大小
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
