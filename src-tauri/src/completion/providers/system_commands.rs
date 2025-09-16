//! 系统命令补全提供者
//!
//! 提供系统PATH中可执行命令的补全

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use async_trait::async_trait;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// 系统命令补全提供者
pub struct SystemCommandsProvider {
    /// 缓存的命令列表
    commands: Arc<RwLock<HashSet<String>>>,
    /// 模糊匹配器
    matcher: SkimMatcherV2,
    /// 是否已初始化
    initialized: Arc<RwLock<bool>>,
}

impl SystemCommandsProvider {
    /// 创建新的系统命令提供者
    pub fn new() -> Self {
        Self {
            commands: Arc::new(RwLock::new(HashSet::new())),
            matcher: SkimMatcherV2::default(),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// 初始化命令列表（扫描PATH）
    pub async fn initialize(&self) -> AppResult<()> {
        let mut initialized = self.initialized.write().await;

        if *initialized {
            return Ok(());
        }

        let mut commands = self.commands.write().await;

        let path_var = env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_var
            .split(if cfg!(windows) { ';' } else { ':' })
            .collect();

        for path_str in paths {
            if path_str.is_empty() {
                continue;
            }

            let path = Path::new(path_str);
            if !path.exists() || !path.is_dir() {
                continue;
            }

            // 读取目录中的可执行文件
            if let Ok(mut entries) = fs::read_dir(path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let entry_path = entry.path();

                    if let Some(file_name) = entry_path.file_name() {
                        let name = file_name.to_string_lossy().to_string();

                        if self.is_executable(&entry_path).await {
                            commands.insert(name);
                        }
                    }
                }
            }
        }

        *initialized = true;

        Ok(())
    }

    /// 检查文件是否可执行
    async fn is_executable(&self, path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(path).await {
            if metadata.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = metadata.permissions();
                    return permissions.mode() & 0o111 != 0;
                }

                #[cfg(windows)]
                {
                    // 在Windows上，检查文件扩展名
                    if let Some(extension) = path.extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        return matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com");
                    }
                }

                #[cfg(not(any(unix, windows)))]
                {
                    return true; // 其他平台默认认为是可执行的
                }
            }
        }

        false
    }

    /// 获取匹配的命令 - 使用模糊匹配
    async fn get_matching_commands(&self, pattern: &str) -> AppResult<Vec<CompletionItem>> {
        let commands = self.commands.read().await;

        let mut matches = Vec::new();

        for command in commands.iter() {
            // 使用模糊匹配器进行匹配，但排除完全相同的命令（没有补全价值）
            if command != pattern {
                if let Some(score) = self.matcher.fuzzy_match(command, pattern) {
                    let item = CompletionItem::new(command.clone(), CompletionType::Command)
                        .with_score(score as f64)
                        .with_description(format!("系统命令: {command}"))
                        .with_source("system_commands".to_string());

                    matches.push(item);
                }
            }
        }

        // 按匹配分数排序（分数高的在前）
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// 检查命令是否存在
    pub async fn has_command(&self, command: &str) -> bool {
        let commands = self.commands.read().await;
        commands.contains(command)
    }

    /// 获取所有命令的数量
    pub async fn command_count(&self) -> usize {
        let commands = self.commands.read().await;
        commands.len()
    }

    /// 手动添加命令（用于测试或特殊情况）
    pub async fn add_command(&self, command: String) -> AppResult<()> {
        let mut commands = self.commands.write().await;

        commands.insert(command);
        Ok(())
    }
}

#[async_trait]
impl CompletionProvider for SystemCommandsProvider {
    fn name(&self) -> &'static str {
        "system_commands"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        let parts: Vec<&str> = context.input.split_whitespace().collect();

        if parts.is_empty() {
            return false;
        }

        let is_first_word =
            parts.len() == 1 || (parts.len() > 1 && context.cursor_position <= parts[0].len());

        is_first_word
            && !context.current_word.contains('/')
            && !context.current_word.contains('\\')
            && !context.current_word.starts_with('.')
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        // 确保已初始化
        self.initialize().await?;

        if context.current_word.is_empty() {
            return Ok(Vec::new());
        }

        self.get_matching_commands(&context.current_word).await
    }

    fn priority(&self) -> i32 {
        8 // 系统命令优先级较高
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for SystemCommandsProvider {
    fn default() -> Self {
        Self::new()
    }
}
