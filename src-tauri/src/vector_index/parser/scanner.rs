/*!
 * 代码文件扫描器
 *
 * 实现递归目录扫描，支持文件扩展名过滤和ignore patterns。
 * 遵循.gitignore规则，添加文件大小和数量限制。
 *
 * Requirements: 1.1, 1.4
 */

use anyhow::{ensure, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::{DirEntry, WalkDir};

use crate::vector_index::types::{Language, VectorIndexFullConfig};

/// 文件扫描统计信息
#[derive(Debug, Clone)]
pub struct ScanStats {
    /// 扫描的总文件数
    pub total_files: usize,
    /// 过滤后的有效文件数
    pub valid_files: usize,
    /// 跳过的文件数（被忽略、太大等）
    pub skipped_files: usize,
    /// 扫描的目录数
    pub directories_scanned: usize,
    /// 跳过的目录数
    pub directories_skipped: usize,
}

/// 代码文件扫描器
pub struct CodeFileScanner {
    /// 支持的文件扩展名集合（小写）
    supported_extensions: HashSet<String>,
    /// 忽略模式（glob patterns）
    ignore_patterns: Vec<glob::Pattern>,
    /// 最大文件大小（字节）
    max_file_size: u64,
    /// 最大文件数量限制
    max_files: Option<usize>,
}

impl CodeFileScanner {
    /// 创建新的文件扫描器
    pub fn new(config: VectorIndexFullConfig) -> Result<Self> {
        // 处理支持的扩展名（去掉前导点并转换为小写）
        let supported_extensions: HashSet<String> = config
            .supported_extensions()
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_lowercase())
            .collect();

        // 编译ignore patterns为glob模式
        let mut ignore_patterns = Vec::new();
        for pattern in config.ignore_patterns() {
            match glob::Pattern::new(pattern) {
                Ok(compiled_pattern) => ignore_patterns.push(compiled_pattern),
                Err(e) => {
                    warn!("无效的ignore pattern '{}': {}", pattern, e);
                }
            }
        }

        // 默认最大文件大小：10MB
        let max_file_size = 10 * 1024 * 1024;

        Ok(Self {
            supported_extensions,
            ignore_patterns,
            max_file_size,
            max_files: None,
        })
    }

    /// 设置最大文件数量限制
    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// 设置最大文件大小限制
    pub fn with_max_file_size(mut self, max_size: u64) -> Self {
        self.max_file_size = max_size;
        self
    }

    /// 扫描指定目录，返回符合条件的代码文件路径列表
    pub async fn scan_directory(&self, root_path: &str) -> Result<(Vec<String>, ScanStats)> {
        let root = PathBuf::from(root_path);
        ensure!(root.exists(), "扫描目录不存在: {}", root_path);
        ensure!(root.is_dir(), "指定路径不是目录: {}", root_path);

        info!("开始扫描目录: {}", root_path);

        let mut valid_files = Vec::new();
        let mut stats = ScanStats {
            total_files: 0,
            valid_files: 0,
            skipped_files: 0,
            directories_scanned: 0,
            directories_skipped: 0,
        };

        // 使用WalkDir进行递归遍历
        let walker = WalkDir::new(&root)
            .follow_links(false) // 不跟随符号链接避免循环
            .min_depth(1); // 跳过根目录本身

        for entry in walker {
            match entry {
                Ok(dir_entry) => {
                    if self.should_stop_scanning(&valid_files) {
                        info!("达到文件数量限制，停止扫描");
                        break;
                    }

                    if dir_entry.file_type().is_dir() {
                        if self.should_skip_directory(&dir_entry, &root)? {
                            stats.directories_skipped += 1;
                        } else {
                            stats.directories_scanned += 1;
                        }
                        continue;
                    }

                    if dir_entry.file_type().is_file() {
                        stats.total_files += 1;

                        if self.should_include_file(&dir_entry, &root).await? {
                            valid_files.push(dir_entry.path().to_string_lossy().to_string());
                            stats.valid_files += 1;
                        } else {
                            stats.skipped_files += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("访问文件失败: {}", e);
                    stats.skipped_files += 1;
                }
            }
        }

        info!(
            "扫描完成: 总文件数={}, 有效文件数={}, 跳过文件数={}",
            stats.total_files, stats.valid_files, stats.skipped_files
        );

        Ok((valid_files, stats))
    }

    /// 检查是否应该停止扫描（达到文件数量限制）
    fn should_stop_scanning(&self, current_files: &[String]) -> bool {
        if let Some(max_files) = self.max_files {
            current_files.len() >= max_files
        } else {
            false
        }
    }

    /// 检查是否应该跳过目录
    fn should_skip_directory(&self, dir_entry: &DirEntry, root: &Path) -> Result<bool> {
        let path = dir_entry.path();

        // 获取相对于根目录的路径
        let relative_path = path
            .strip_prefix(root)
            .with_context(|| format!("无法计算相对路径: {:?}", path))?;

        // 检查是否匹配ignore patterns
        if self.matches_ignore_patterns(relative_path) {
            debug!("跳过目录（匹配ignore pattern）: {:?}", relative_path);
            return Ok(true);
        }

        // 检查常见的隐藏目录
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name.starts_with('.') && dir_name != "." && dir_name != ".." {
                debug!("跳过隐藏目录: {}", dir_name);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 检查文件是否应该包含在扫描结果中
    async fn should_include_file(&self, dir_entry: &DirEntry, root: &Path) -> Result<bool> {
        let path = dir_entry.path();

        // 获取相对于根目录的路径
        let relative_path = path
            .strip_prefix(root)
            .with_context(|| format!("无法计算相对路径: {:?}", path))?;

        // 检查是否匹配ignore patterns
        if self.matches_ignore_patterns(relative_path) {
            debug!("跳过文件（匹配ignore pattern）: {:?}", relative_path);
            return Ok(false);
        }

        // 检查文件扩展名
        if !self.has_supported_extension(path) {
            return Ok(false);
        }

        // 检查文件大小
        match dir_entry.metadata() {
            Ok(metadata) => {
                if metadata.len() > self.max_file_size {
                    debug!(
                        "跳过文件（文件过大: {}字节）: {:?}",
                        metadata.len(),
                        relative_path
                    );
                    return Ok(false);
                }
            }
            Err(e) => {
                warn!("获取文件元数据失败: {:?} - {}", path, e);
                return Ok(false);
            }
        }

        // 检查文件是否可读
        match tokio::fs::File::open(path).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("文件无法打开: {:?} - {}", path, e);
                Ok(false)
            }
        }
    }

    /// 检查文件是否有支持的扩展名
    fn has_supported_extension(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            self.supported_extensions.contains(&ext_lower)
        } else {
            false
        }
    }

    /// 检查路径是否匹配ignore patterns
    fn matches_ignore_patterns(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.ignore_patterns {
            if pattern.matches(&path_str) {
                return true;
            }

            // 也检查路径的各个组件
            for component in path.components() {
                if let Some(component_str) = component.as_os_str().to_str() {
                    if pattern.matches(component_str) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// 检测文件的编程语言
    pub fn detect_file_language(&self, file_path: &str) -> Option<Language> {
        let path = Path::new(file_path);
        let extension = path.extension()?.to_str()?;
        Language::from_extension(extension)
    }

    /// 根据语言过滤文件列表
    pub fn filter_by_language(&self, files: &[String], language: Language) -> Vec<String> {
        files
            .iter()
            .filter_map(|file_path| {
                if self.detect_file_language(file_path) == Some(language) {
                    Some(file_path.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// 按语言分组文件
    pub fn group_files_by_language(
        &self,
        files: &[String],
    ) -> std::collections::HashMap<Language, Vec<String>> {
        let mut grouped = std::collections::HashMap::new();

        for file_path in files {
            if let Some(language) = self.detect_file_language(file_path) {
                grouped
                    .entry(language)
                    .or_insert_with(Vec::new)
                    .push(file_path.clone());
            }
        }

        grouped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_structure() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建测试文件结构
        fs::create_dir_all(root.join("src")).await?;
        fs::create_dir_all(root.join("tests")).await?;
        fs::create_dir_all(root.join("node_modules")).await?;
        fs::create_dir_all(root.join(".git")).await?;

        // 创建测试文件
        fs::write(root.join("src/main.rs"), "fn main() {}").await?;
        fs::write(root.join("src/lib.rs"), "pub fn test() {}").await?;
        fs::write(root.join("tests/test.py"), "def test(): pass").await?;
        fs::write(root.join("node_modules/index.js"), "console.log('test')").await?;
        fs::write(root.join(".git/config"), "[core]").await?;
        fs::write(root.join("README.md"), "# Test").await?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_basic_file_scanning() -> Result<()> {
        let temp_dir = create_test_structure().await?;
        let config = VectorIndexFullConfig::default();
        let scanner = CodeFileScanner::new(config)?;

        let (files, stats) = scanner
            .scan_directory(temp_dir.path().to_str().unwrap())
            .await?;

        // 应该找到支持的代码文件，但不包括被忽略的文件
        assert!(files.len() > 0);
        assert!(stats.valid_files > 0);
        assert!(!files.iter().any(|f| f.contains("node_modules")));
        assert!(!files.iter().any(|f| f.contains(".git")));

        Ok(())
    }

    #[tokio::test]
    async fn test_language_detection() -> Result<()> {
        let config = VectorIndexFullConfig::default();
        let scanner = CodeFileScanner::new(config)?;

        assert_eq!(
            scanner.detect_file_language("test.rs"),
            Some(Language::Rust)
        );
        assert_eq!(
            scanner.detect_file_language("test.py"),
            Some(Language::Python)
        );
        assert_eq!(
            scanner.detect_file_language("test.ts"),
            Some(Language::TypeScript)
        );
        assert_eq!(scanner.detect_file_language("test.md"), None);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_size_limit() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // 创建一个大文件
        let large_content = "x".repeat(1024 * 1024); // 1MB
        fs::write(root.join("large.rs"), large_content).await?;
        fs::write(root.join("small.rs"), "fn test() {}").await?;

        let config = VectorIndexFullConfig::default();
        let scanner = CodeFileScanner::new(config)?.with_max_file_size(500 * 1024); // 500KB限制

        let (files, _) = scanner.scan_directory(root.to_str().unwrap()).await?;

        // 应该只包含小文件
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("small.rs"));

        Ok(())
    }
}
