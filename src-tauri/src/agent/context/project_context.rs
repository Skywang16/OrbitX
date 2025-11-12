/*!
 * Project Context Loader
 *
 * 按优先级读取项目文档，注入到 Agent 上下文
 * 优先级顺序：CLAUDE.md > AGENTS.md > WARP.md > .cursorrules > README.md
 */

use std::path::PathBuf;
use tokio::fs;

/// 项目上下文配置文件优先级列表
const CONTEXT_FILES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "WARP.md",
    ".cursorrules",
    "README.md",
];

/// 获取所有可用的规则文件列表
pub fn get_available_rules_files<P: Into<PathBuf>>(project_root: P) -> Vec<String> {
    let root: PathBuf = project_root.into();
    CONTEXT_FILES
        .iter()
        .filter_map(|&filename| {
            let file_path = root.join(filename);
            if file_path.exists() {
                Some(filename.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 项目上下文加载器
pub struct ProjectContextLoader {
    project_root: PathBuf,
}

impl ProjectContextLoader {
    /// 创建新的加载器实例
    pub fn new<P: Into<PathBuf>>(project_root: P) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// 按优先级加载第一个存在的项目文档
    pub async fn load_context(&self) -> Option<ProjectContext> {
        self.load_with_preference(None).await
    }

    /// 按指定偏好加载项目文档，如果未指定或文件不存在则按默认优先级
    pub async fn load_with_preference(
        &self,
        preferred_file: Option<&str>,
    ) -> Option<ProjectContext> {
        // 如果指定了偏好文件，优先尝试加载
        if let Some(pref) = preferred_file {
            if let Some(ctx) = self.try_load_file(pref).await {
                return Some(ctx);
            }
        }

        // 按默认优先级尝试加载
        for filename in CONTEXT_FILES {
            if let Some(ctx) = self.try_load_file(filename).await {
                return Some(ctx);
            }
        }

        None
    }

    /// 尝试加载单个文件
    async fn try_load_file(&self, filename: &str) -> Option<ProjectContext> {
        let file_path = self.project_root.join(filename);

        if !file_path.exists() {
            return None;
        }

        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    return None;
                }


                Some(ProjectContext {
                    source_file: filename.to_string(),
                    content: trimmed.to_string(),
                })
            }
            Err(_) => None,
        }
    }
}

/// 项目上下文数据
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// 源文件名（例如 "CLAUDE.md"）
    pub source_file: String,
    /// 文件内容
    pub content: String,
}

impl ProjectContext {
    /// 格式化为注入到 System Prompt 的文本
    pub fn format_for_prompt(&self) -> String {
        format!(
            "# Project Context (from {})\n\n{}",
            self.source_file, self.content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_highest_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 创建多个文件，应该只读取最高优先级的 CLAUDE.md
        std::fs::write(temp_path.join("CLAUDE.md"), "Claude instructions").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "CLAUDE.md");
        assert_eq!(ctx.content, "Claude instructions");
    }

    #[tokio::test]
    async fn test_fallback_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 只创建 WARP.md 和 README.md，应该读取 WARP.md
        std::fs::write(temp_path.join("WARP.md"), "Warp config").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "WARP.md");
        assert_eq!(ctx.content, "Warp config");
    }

    #[tokio::test]
    async fn test_no_context_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_none());
    }

    #[tokio::test]
    async fn test_skip_empty_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // CLAUDE.md 为空，应该跳过并读取 README.md
        std::fs::write(temp_path.join("CLAUDE.md"), "   \n  \n  ").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "README.md");
    }
}
