//! 历史命令补全提供者
//!
//! 基于用户的命令历史提供补全建议

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::cache::UnifiedCache;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;

/// Shell类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Unknown,
}

impl ShellType {
    /// 从文件路径推断Shell类型
    fn from_path(path: &PathBuf) -> Self {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            match file_name {
                ".bash_history" => Self::Bash,
                ".zsh_history" => Self::Zsh,
                ".fish_history" => Self::Fish,
                _ => Self::Unknown,
            }
        } else {
            Self::Unknown
        }
    }
}

/// 历史命令补全提供者
pub struct HistoryProvider {
    /// 历史文件路径
    history_file: Option<PathBuf>,
    /// 最大历史记录数
    max_entries: usize,
    /// 统一缓存
    cache: Arc<UnifiedCache>,
    /// 当前Shell类型
    shell_type: ShellType,
}

impl HistoryProvider {
    /// 创建新的历史提供者
    pub fn new(cache: Arc<UnifiedCache>) -> Self {
        let mut provider = Self {
            history_file: None,
            max_entries: 1000,
            cache,
            shell_type: ShellType::Unknown,
        };

        // 尝试设置默认的历史文件路径
        if let Some(home_dir) = dirs::home_dir() {
            let bash_history = home_dir.join(".bash_history");
            let zsh_history = home_dir.join(".zsh_history");
            let fish_history = home_dir.join(".local/share/fish/fish_history");

            if bash_history.exists() {
                provider = provider.with_history_file(bash_history);
            } else if zsh_history.exists() {
                provider = provider.with_history_file(zsh_history);
            } else if fish_history.exists() {
                provider = provider.with_history_file(fish_history);
            }
        }

        provider
    }

    /// 设置历史文件路径
    pub fn with_history_file(mut self, path: PathBuf) -> Self {
        self.shell_type = ShellType::from_path(&path);
        self.history_file = Some(path);
        self
    }

    /// 设置最大历史记录数
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// 从文件读取历史记录
    async fn read_history(&self) -> CompletionProviderResult<Vec<String>> {
        let cache_key = "completion:history:commands";

        // 尝试从缓存获取
        if let Some(cached_value) = self.cache.get(cache_key).await {
            if let Ok(commands) = serde_json::from_value::<Vec<String>>(cached_value) {
                return Ok(commands);
            }
        }

        // 缓存未命中，从文件读取
        if let Some(history_file) = &self.history_file {
            if history_file.exists() {
                let content = fs::read_to_string(history_file)
                    .await
                    .map_err(|e| CompletionProviderError::io(
                        "read history file",
                        format!("({})", history_file.display()),
                        e,
                    ))?;
                let commands = self.parse_history_content(&content);

                // 存入缓存
                if let Ok(commands_value) = serde_json::to_value(&commands) {
                    let _ = self.cache.set(cache_key, commands_value).await;
                }

                return Ok(commands);
            }
        }

        Ok(Vec::new())
    }

    /// 解析历史文件内容，支持不同的shell格式
    fn parse_history_content(&self, content: &str) -> Vec<String> {
        let mut commands = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 根据Shell类型解析不同格式
            let command = match self.shell_type {
                ShellType::Zsh => {
                    if line.starts_with(": ") && line.contains(';') {
                        line.find(';')
                            .map(|pos| line[pos + 1..].trim())
                            .filter(|cmd| !cmd.is_empty())
                            .map(|cmd| cmd.to_string())
                    } else {
                        Some(line.to_string())
                    }
                }
                ShellType::Fish => {
                    // Fish历史格式通常是YAML，这里简化处理
                    if line.starts_with("- cmd: ") {
                        Some(line[8..].trim().to_string())
                    } else {
                        None
                    }
                }
                _ => {
                    // Bash和其他格式：直接是命令
                    Some(line.to_string())
                }
            };

            if let Some(cmd) = command {
                commands.push(cmd);
            }
        }

        // 去重并保持最近的命令
        self.deduplicate_commands(commands)
    }

    /// 去重命令，保持最新的
    fn deduplicate_commands(&self, commands: Vec<String>) -> Vec<String> {
        let mut unique_commands = Vec::new();
        let mut seen = HashSet::new();

        // 从后往前遍历，保持最新的命令
        for command in commands.into_iter().rev().take(self.max_entries) {
            if seen.insert(command.clone()) {
                unique_commands.push(command);
            }
        }

        unique_commands.reverse();
        unique_commands
    }

    /// 获取匹配的历史命令
    async fn get_matching_commands(
        &self,
        pattern: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let commands = self.read_history().await?;
        let mut matches = Vec::new();

        for (index, command) in commands.iter().enumerate() {
            if self.is_command_match(command, pattern) {
                let score = self.calculate_command_score(command, pattern, index);
                let item = CompletionItem::new(command.clone(), CompletionType::History)
                    .with_score(score)
                    .with_description(format!("历史命令 ({})", self.shell_type_name()))
                    .with_source("history".to_string());
                matches.push(item);
            }
        }

        // 按分数排序
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// 检查命令是否匹配模式
    fn is_command_match(&self, command: &str, pattern: &str) -> bool {
        command.starts_with(pattern) && command != pattern
    }

    /// 计算命令分数
    fn calculate_command_score(&self, command: &str, pattern: &str, index: usize) -> f64 {
        let mut score = 70.0;

        // 匹配长度加分
        let match_ratio = pattern.len() as f64 / command.len() as f64;
        score += match_ratio * 20.0;

        // 位置加分（越靠前的历史命令分数越高）
        let position_bonus = (1000 - index.min(1000)) as f64 / 1000.0 * 10.0;
        score += position_bonus;

        score
    }

    /// 获取Shell类型名称
    fn shell_type_name(&self) -> &'static str {
        match self.shell_type {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::Unknown => "shell",
        }
    }
}

#[async_trait]
impl CompletionProvider for HistoryProvider {
    fn name(&self) -> &'static str {
        "history"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        if self.history_file.is_none() {
            return false;
        }

        let word = &context.current_word;
        !word.is_empty()
            && !word.starts_with('/')
            && !word.starts_with('\\')
            && !word.starts_with('.')
            && !word.starts_with('-')  // 排除选项参数
            && word.len() >= 2 // 至少2个字符才开始历史补全
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        if context.current_word.is_empty() {
            return Ok(Vec::new());
        }

        self.get_matching_commands(&context.current_word).await
    }

    fn priority(&self) -> i32 {
        15 // 历史命令优先级最高 - 智能学习用户习惯
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::cache::UnifiedCache;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_shell_type_detection() {
        let bash_path = PathBuf::from("/home/user/.bash_history");
        let zsh_path = PathBuf::from("/home/user/.zsh_history");
        let fish_path = PathBuf::from("/home/user/.fish_history");
        let unknown_path = PathBuf::from("/home/user/.unknown_history");

        assert_eq!(ShellType::from_path(&bash_path), ShellType::Bash);
        assert_eq!(ShellType::from_path(&zsh_path), ShellType::Zsh);
        assert_eq!(ShellType::from_path(&fish_path), ShellType::Fish);
        assert_eq!(ShellType::from_path(&unknown_path), ShellType::Unknown);
    }

    #[tokio::test]
    async fn test_bash_history_parsing() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        let bash_content = r#"ls -la
cd /home/user
git status
npm install
ls -la
git commit -m "test"
"#;

        let commands = provider.parse_history_content(bash_content);

        // 应该去重，保留最新的命令
        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"cd /home/user".to_string()));
        assert!(commands.contains(&"git status".to_string()));
        assert!(commands.contains(&"npm install".to_string()));
        assert!(commands.contains(&"git commit -m \"test\"".to_string()));

        let ls_count = commands.iter().filter(|&cmd| cmd == "ls -la").count();
        assert_eq!(ls_count, 1);
    }

    #[tokio::test]
    async fn test_zsh_history_parsing() {
        let cache = Arc::new(UnifiedCache::new());
        let zsh_path = PathBuf::from(".zsh_history");
        let provider = HistoryProvider::new(cache).with_history_file(zsh_path);

        let zsh_content = r#": 1640995200:0;ls -la
: 1640995201:0;cd /home/user
: 1640995202:0;git status
# comment line
: 1640995203:0;npm install
"#;

        let commands = provider.parse_history_content(zsh_content);

        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"cd /home/user".to_string()));
        assert!(commands.contains(&"git status".to_string()));
        assert!(commands.contains(&"npm install".to_string()));
        assert_eq!(commands.len(), 4);
    }

    #[tokio::test]
    async fn test_command_matching() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        // 测试匹配逻辑
        assert!(provider.is_command_match("git status", "git"));
        assert!(provider.is_command_match("git commit", "git"));
        assert!(!provider.is_command_match("git", "git")); // 完全相同不匹配
        assert!(!provider.is_command_match("ls", "git")); // 不匹配
    }

    #[tokio::test]
    async fn test_score_calculation() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        let score1 = provider.calculate_command_score("git status", "git", 0);
        let score2 = provider.calculate_command_score("git status", "git", 10);
        let score3 = provider.calculate_command_score("git", "g", 0);

        // 位置越靠前分数越高
        assert!(score1 > score2);

        // 匹配度越高分数越高
        assert!(score3 > score1); // "g" 匹配 "git" 的比例更高
    }

    #[tokio::test]
    async fn test_should_provide_logic() {
        let cache = Arc::new(UnifiedCache::new());
        let bash_path = PathBuf::from(".bash_history");
        let provider = HistoryProvider::new(cache).with_history_file(bash_path);

        let context_valid =
            CompletionContext::new("git".to_string(), 3, PathBuf::from("/home/user"));

        let context_too_short =
            CompletionContext::new("g".to_string(), 1, PathBuf::from("/home/user"));

        let context_path =
            CompletionContext::new("/home".to_string(), 5, PathBuf::from("/home/user"));

        assert!(provider.should_provide(&context_valid));
        assert!(!provider.should_provide(&context_too_short));
        assert!(!provider.should_provide(&context_path));
    }
}
