/*!
 * 文件过滤器模块
 *
 * 提供统一的文件过滤逻辑，避免在多个模块中重复相同的过滤代码。
 * 支持扩展名过滤、路径忽略模式匹配和文件大小限制。
 *
 * ## 主要功能
 *
 * - **扩展名过滤**: 根据支持的文件扩展名过滤文件
 * - **路径忽略**: 支持glob模式的路径忽略规则
 * - **文件大小限制**: 避免处理过大的文件
 * - **统一接口**: 为监控服务和检测器提供统一的过滤逻辑
 *
 * ## 设计原则
 *
 * - 单一职责：专注文件过滤逻辑
 * - 性能优化：缓存编译的glob模式
 * - 配置驱动：基于VectorIndexConfig进行过滤
 */

use std::collections::HashSet;
use std::path::Path;

use crate::vector_index::types::VectorIndexConfig;

/// 文件过滤器
pub struct FileFilter {
    /// 支持的文件扩展名集合
    supported_extensions: HashSet<String>,
    /// 忽略模式列表
    ignore_patterns: Vec<String>,
    /// 最大文件大小（字节）
    max_file_size: u64,
}

impl FileFilter {
    /// 创建新的文件过滤器
    pub fn new(config: &VectorIndexConfig) -> Self {
        let supported_extensions: HashSet<String> = config
            .supported_extensions
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_lowercase())
            .collect();

        Self {
            supported_extensions,
            ignore_patterns: config.ignore_patterns.clone(),
            max_file_size: 5 * 1024 * 1024, // 5MB
        }
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, max_size: u64) -> Self {
        self.max_file_size = max_size;
        self
    }

    /// 判断是否应该处理该文件
    pub fn should_process_file(&self, path: &Path) -> bool {
        // 1. 检查是否是文件
        if !path.is_file() {
            return false;
        }

        // 2. 检查文件扩展名
        if !self.has_supported_extension(path) {
            return false;
        }

        // 3. 检查忽略模式
        if self.is_path_ignored(path) {
            return false;
        }

        // 4. 检查文件大小
        if !self.is_file_size_acceptable(path) {
            return false;
        }

        true
    }

    /// 检查文件是否有支持的扩展名
    fn has_supported_extension(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            self.supported_extensions
                .contains(&extension.to_lowercase())
        } else {
            false
        }
    }

    /// 检查路径是否被忽略
    fn is_path_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // 检查配置的忽略模式
        for pattern in &self.ignore_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(&path_str) {
                    return true;
                }
            }
        }

        // 检查常见的忽略模式
        let common_ignore_patterns = [
            ".git",
            ".gitignore",
            ".gitmodules",
            "node_modules",
            "target",
            "dist",
            "build",
            ".DS_Store",
            "Thumbs.db",
            "*.tmp",
            "*.log",
            "*.lock",
            "*.swp",
            "*.bak",
            ".vscode",
            ".idea",
        ];

        for pattern in &common_ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 检查文件大小是否可接受
    fn is_file_size_acceptable(&self, path: &Path) -> bool {
        match path.metadata() {
            Ok(metadata) => {
                if metadata.len() > self.max_file_size {
                    tracing::debug!(
                        "跳过过大文件: {} ({}B > {}B)",
                        path.display(),
                        metadata.len(),
                        self.max_file_size
                    );
                    false
                } else {
                    true
                }
            }
            Err(_) => {
                // 无法获取文件元数据，谨慎起见跳过
                false
            }
        }
    }

    /// 获取支持的扩展名
    pub fn get_supported_extensions(&self) -> &HashSet<String> {
        &self.supported_extensions
    }

    /// 获取忽略模式
    pub fn get_ignore_patterns(&self) -> &[String] {
        &self.ignore_patterns
    }

    /// 获取最大文件大小
    pub fn get_max_file_size(&self) -> u64 {
        self.max_file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_file_filter_creation() {
        let config = VectorIndexConfig::default();
        let filter = FileFilter::new(&config);

        // 验证基本属性
        assert!(!filter.supported_extensions.is_empty());
        assert!(filter.supported_extensions.contains("ts"));
        assert!(filter.supported_extensions.contains("rs"));
        assert_eq!(filter.max_file_size, 5 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_should_process_file() {
        let config = VectorIndexConfig::default();
        let filter = FileFilter::new(&config);

        // 创建测试文件
        let temp_dir = tempdir().unwrap();
        let ts_file = temp_dir.path().join("test.ts");
        let txt_file = temp_dir.path().join("test.txt");
        let git_file = temp_dir.path().join(".git").join("config");

        fs::write(&ts_file, "console.log('test');").await.unwrap();
        fs::write(&txt_file, "plain text").await.unwrap();

        // 测试支持的文件类型
        assert!(filter.should_process_file(&ts_file));

        // 测试不支持的文件类型
        assert!(!filter.should_process_file(&txt_file));

        // 测试目录
        assert!(!filter.should_process_file(temp_dir.path()));

        // 测试忽略的路径
        assert!(!filter.should_process_file(&git_file));
    }

    #[tokio::test]
    async fn test_file_size_filtering() {
        let config = VectorIndexConfig::default();
        let filter = FileFilter::new(&config).with_max_file_size(100); // 100字节限制

        let temp_dir = tempdir().unwrap();
        let small_file = temp_dir.path().join("small.ts");
        let large_file = temp_dir.path().join("large.ts");

        // 创建小文件
        fs::write(&small_file, "small").await.unwrap();
        
        // 创建大文件
        let large_content = "a".repeat(200);
        fs::write(&large_file, large_content).await.unwrap();

        // 验证大小过滤
        assert!(filter.should_process_file(&small_file));
        assert!(!filter.should_process_file(&large_file));
    }

    #[test]
    fn test_ignore_patterns() {
        let config = VectorIndexConfig::default();
        let filter = FileFilter::new(&config);

        // 测试常见忽略模式
        let test_paths = [
            "/project/node_modules/package/index.js",
            "/project/.git/config",
            "/project/target/debug/main",
            "/project/dist/bundle.js",
            "/project/.DS_Store",
            "/project/.vscode/settings.json",
        ];

        for path_str in &test_paths {
            let path = PathBuf::from(path_str);
            assert!(
                filter.is_path_ignored(&path),
                "路径应该被忽略: {}",
                path_str
            );
        }

        // 测试不应该被忽略的路径
        let normal_paths = [
            "/project/src/main.ts",
            "/project/lib/utils.rs",
            "/project/components/Button.vue",
        ];

        for path_str in &normal_paths {
            let path = PathBuf::from(path_str);
            assert!(
                !filter.is_path_ignored(&path),
                "路径不应该被忽略: {}",
                path_str
            );
        }
    }
}
