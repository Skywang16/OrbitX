use crate::ai::tool::ast::parser::AstParser;
use crate::ai::tool::ast::types::{AnalysisResult, AnalyzeCodeParams};
use crate::utils::error::ToTauriResult;
use anyhow::{Context, Result};
use std::path::Path;
use tauri::command;
use walkdir::WalkDir;

#[command]
pub async fn analyze_code(params: AnalyzeCodeParams) -> Result<AnalysisResult, String> {
    let analyzer = CodeAnalyzer::new();
    analyzer.analyze(params).await.to_tauri()
}

pub struct CodeAnalyzer {
    parser: AstParser,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {
            parser: AstParser::new(),
        }
    }

    pub async fn analyze(&self, params: AnalyzeCodeParams) -> Result<AnalysisResult> {
        let path = Path::new(&params.path);

        if !path.exists() {
            anyhow::bail!("文件不存在: {}", params.path);
        }

        let files_to_analyze = if path.is_file() {
            vec![params.path]
        } else {
            self.collect_files(&params)?
        };

        let mut analyses = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;

        for file_path in &files_to_analyze {
            match self.parser.analyze_file(file_path).await {
                Ok(analysis) => {
                    tracing::debug!(
                        "成功分析文件: {} (找到 {} 个符号)",
                        file_path,
                        analysis.symbols.len()
                    );
                    analyses.push(analysis);
                    success_count += 1;
                }
                Err(e) => {
                    tracing::warn!("分析文件 {} 失败: {}", file_path, e);
                    error_count += 1;
                }
            }
        }

        Ok(AnalysisResult {
            analyses,
            total_files: files_to_analyze.len(),
            success_count,
            error_count,
        })
    }

    fn collect_files(&self, params: &AnalyzeCodeParams) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let recursive = params.recursive.unwrap_or(false);
        let default_include = vec![];
        let default_exclude = vec![];
        let include_patterns = params.include.as_ref().unwrap_or(&default_include);
        let exclude_patterns = params.exclude.as_ref().unwrap_or(&default_exclude);

        let walker = if recursive {
            WalkDir::new(&params.path).into_iter()
        } else {
            WalkDir::new(&params.path).max_depth(1).into_iter()
        };

        for entry in walker {
            let entry = entry.with_context(|| format!("遍历目录失败: {}", params.path))?;

            if entry.file_type().is_file() {
                let file_path = entry.path().to_string_lossy().to_string();
                let file_name = entry.file_name().to_string_lossy().to_string();

                if self.should_include(&file_name, include_patterns)
                    && !self.should_exclude(&file_name, exclude_patterns)
                {
                    files.push(file_path);
                }
            }
        }

        Ok(files)
    }

    fn should_include(&self, file_name: &str, include_patterns: &[String]) -> bool {
        if include_patterns.is_empty() {
            // 默认包含常见的代码文件
            let supported_extensions = [".ts", ".tsx", ".js", ".jsx", ".py", ".rs"];
            return supported_extensions
                .iter()
                .any(|ext| file_name.ends_with(ext));
        }

        include_patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                self.match_glob_pattern(file_name, pattern)
            } else {
                file_name.contains(pattern)
            }
        })
    }

    fn should_exclude(&self, file_name: &str, exclude_patterns: &[String]) -> bool {
        exclude_patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                self.match_glob_pattern(file_name, pattern)
            } else {
                file_name.contains(pattern)
            }
        })
    }

    fn match_glob_pattern(&self, text: &str, pattern: &str) -> bool {
        // 简单的 glob 模式匹配
        let regex_pattern = pattern.replace("*", ".*");
        if let Ok(regex) = regex::Regex::new(&regex_pattern) {
            regex.is_match(text)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_analyze_typescript_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.ts");

        let content = r#"
interface User {
    name: string;
    age: number;
}

class UserService {
    getUser(): User {
        return { name: "test", age: 25 };
    }
}

function createUser(name: string): User {
    return { name, age: 0 };
}

export { UserService, createUser };
"#;

        fs::write(&file_path, content).await.unwrap();

        let analyzer = CodeAnalyzer::new();
        let params = AnalyzeCodeParams {
            path: file_path.to_string_lossy().to_string(),
            recursive: Some(false),
            include: None,
            exclude: None,
        };

        let result = analyzer.analyze(params).await.unwrap();

        assert_eq!(result.analyses.len(), 1);
        let analysis = &result.analyses[0];
        assert_eq!(analysis.language, "typescript");
        // 解析器当前提取 class 与 function（不包含 interface）
        assert!(analysis.symbols.len() >= 2);

        // 检查是否找到了预期的符号
        let symbol_names: Vec<&String> = analysis.symbols.iter().map(|s| &s.name).collect();
        assert!(symbol_names.contains(&&"UserService".to_string()));
        assert!(symbol_names.contains(&&"createUser".to_string()));
    }

    #[tokio::test]
    async fn test_analyze_directory() {
        let dir = tempdir().unwrap();

        // 创建测试文件
        let ts_file = dir.path().join("test.ts");
        fs::write(&ts_file, "function hello() {}").await.unwrap();

        let js_file = dir.path().join("test.js");
        fs::write(&js_file, "const world = 'world';").await.unwrap();

        let analyzer = CodeAnalyzer::new();
        let params = AnalyzeCodeParams {
            path: dir.path().to_string_lossy().to_string(),
            recursive: Some(false),
            include: None,
            exclude: None,
        };

        let result = analyzer.analyze(params).await.unwrap();

        assert_eq!(result.analyses.len(), 2);
        assert_eq!(result.success_count, 2);
        assert_eq!(result.error_count, 0);
    }
}
