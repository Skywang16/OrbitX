//! 历史命令补全提供者
//!
//! 基于用户的命令历史提供补全建议

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::command_line::extract_command_key;
use crate::completion::scoring::{
    BaseScorer, CompositeScorer, FrecencyScorer, HistoryScorer, ScoreCalculator, ScoringContext,
};
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::cache::UnifiedCache;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;

const HISTORY_TTL_DAYS: u64 = 30;
const KEY_MODE_SCAN_LIMIT: usize = 2000;
const KEY_MODE_MAX_RESULTS: usize = 50;
const FULL_MODE_SCAN_LIMIT: usize = 400;
const PSEUDO_RECENCY_STEP_SECS: u64 = 90;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HistoryEntry {
    command: String,
    last_used_ts: Option<u64>,
}

#[derive(Debug, Clone)]
struct CommandKeyStats {
    count: u64,
    first_index: usize,
    last_used_ts: Option<u64>,
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
    async fn read_history(&self) -> CompletionProviderResult<Vec<HistoryEntry>> {
        let cache_key = "completion:history:commands";

        // 尝试从缓存获取
        if let Some(cached_value) = self.cache.get(cache_key).await {
            if let Ok(entries) = serde_json::from_value::<Vec<HistoryEntry>>(cached_value) {
                return Ok(entries);
            }
        }

        // 缓存未命中，从文件读取
        if let Some(history_file) = &self.history_file {
            if history_file.exists() {
                let content = fs::read_to_string(history_file).await.map_err(|e| {
                    CompletionProviderError::io(
                        "read history file",
                        format!("({})", history_file.display()),
                        e,
                    )
                })?;
                let entries = self.parse_history_content(&content);

                // 存入缓存
                if let Ok(entries_value) = serde_json::to_value(&entries) {
                    let _ = self.cache.set(cache_key, entries_value).await;
                }

                return Ok(entries);
            }
        }

        Ok(Vec::new())
    }

    /// 解析历史文件内容，支持不同的shell格式
    fn parse_history_content(&self, content: &str) -> Vec<HistoryEntry> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 根据Shell类型解析不同格式
            let entry = match self.shell_type {
                ShellType::Zsh => {
                    if line.starts_with(": ") && line.contains(';') {
                        let timestamp = line
                            .split(';')
                            .next()
                            .and_then(|head| head.split(':').nth(1))
                            .and_then(|ts| ts.trim().parse::<u64>().ok());

                        line.find(';')
                            .map(|pos| line[pos + 1..].trim())
                            .filter(|cmd| !cmd.is_empty())
                            .map(|cmd| HistoryEntry {
                                command: cmd.to_string(),
                                last_used_ts: timestamp,
                            })
                    } else {
                        Some(HistoryEntry {
                            command: line.to_string(),
                            last_used_ts: None,
                        })
                    }
                }
                ShellType::Fish => {
                    // Fish历史格式通常是YAML，这里简化处理
                    if line.starts_with("- cmd: ") {
                        Some(HistoryEntry {
                            command: line[8..].trim().to_string(),
                            last_used_ts: None,
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    // Bash和其他格式：直接是命令
                    Some(HistoryEntry {
                        command: line.to_string(),
                        last_used_ts: None,
                    })
                }
            };

            if let Some(entry) = entry {
                entries.push(entry);
            }
        }

        // 去重并保持最近的命令
        self.deduplicate_entries(entries)
    }

    /// 去重命令，保持最新的（返回顺序：新 -> 旧）
    fn deduplicate_entries(&self, entries: Vec<HistoryEntry>) -> Vec<HistoryEntry> {
        let mut unique_entries = Vec::new();
        let mut seen = HashSet::new();

        // 从后往前遍历（文件尾部通常更“新”），保留最新一条；不 reverse，保持新->旧
        for entry in entries.into_iter().rev().take(self.max_entries) {
            if seen.insert(entry.command.clone()) {
                unique_entries.push(entry);
            }
        }

        unique_entries
    }

    /// 获取匹配的历史命令
    async fn get_matching_commands(
        &self,
        pattern: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let entries = self.read_history().await?;
        let mut matches = Vec::new();

        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let cutoff_ts = now_ts.saturating_sub(HISTORY_TTL_DAYS * 24 * 60 * 60);

        if pattern.contains(' ') {
            // 用户已经输入到“参数级”，返回完整命令，但只扫描最近一段，避免老垃圾刷屏。
            for (index, entry) in entries.iter().take(FULL_MODE_SCAN_LIMIT).enumerate() {
                if entry
                    .last_used_ts
                    .is_some_and(|ts| ts < cutoff_ts)
                {
                    continue;
                }

                if self.is_command_match(&entry.command, pattern) {
                    let score =
                        self.calculate_command_score(&entry.command, pattern, index, entry.last_used_ts);
                    matches.push(
                        CompletionItem::new(entry.command.clone(), CompletionType::History)
                            .with_score(score)
                            .with_description(format!("历史命令 ({})", self.shell_type_name()))
                            .with_source("history".to_string()),
                    );
                }
            }
        } else {
            // 只输入了 root（例如 "git"）：不要返回带 hash/path 的整行命令。
            // 这里做 key 聚合（git status / git show / docker ps ...），并带频率+时近性。
            let mut stats: HashMap<String, CommandKeyStats> = HashMap::new();
            for (index, entry) in entries.iter().take(KEY_MODE_SCAN_LIMIT).enumerate() {
                if entry
                    .last_used_ts
                    .is_some_and(|ts| ts < cutoff_ts)
                {
                    continue;
                }

                let Some(key) = extract_command_key(&entry.command) else {
                    continue;
                };

                if !key.key.starts_with(pattern) || key.key == pattern {
                    continue;
                }

                stats
                    .entry(key.key)
                    .and_modify(|s| {
                        s.count = s.count.saturating_add(1);
                        s.first_index = s.first_index.min(index);
                        if s.last_used_ts.is_none() {
                            s.last_used_ts = entry.last_used_ts;
                        }
                    })
                    .or_insert(CommandKeyStats {
                        count: 1,
                        first_index: index,
                        last_used_ts: entry.last_used_ts,
                    });
            }

            for (key, s) in stats {
                let pseudo_ts = now_ts.saturating_sub((s.first_index as u64) * PSEUDO_RECENCY_STEP_SECS);
                let ts = s.last_used_ts.unwrap_or(pseudo_ts);
                let score = self.calculate_command_key_score(pattern, &key, s.first_index, s.count, ts);
                matches.push(
                    CompletionItem::new(key, CompletionType::History)
                        .with_score(score)
                        .with_description(format!("历史命令 ({})", self.shell_type_name()))
                        .with_source("history".to_string()),
                );
            }
        }

        // 按分数排序（使用 CompletionItem 的 Ord 实现）
        matches.sort_unstable();
        matches.truncate(KEY_MODE_MAX_RESULTS);

        Ok(matches)
    }

    /// 检查命令是否匹配模式
    fn is_command_match(&self, command: &str, pattern: &str) -> bool {
        command.starts_with(pattern) && command != pattern
    }

    /// 计算命令分数（使用统一评分系统）
    fn calculate_command_score(
        &self,
        command: &str,
        pattern: &str,
        index: usize,
        last_used_ts: Option<u64>,
    ) -> f64 {
        let is_prefix_match = command.starts_with(pattern);
        let history_weight = Self::calculate_history_weight(index);

        let mut context = ScoringContext::new(pattern, command)
            .with_prefix_match(is_prefix_match)
            .with_history_weight(history_weight)
            .with_history_position(index)
            .with_source("history");

        if let Some(ts) = last_used_ts {
            context = context.with_last_used_timestamp(ts);
        }

        let scorer = CompositeScorer::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(FrecencyScorer),
        ]);

        scorer.calculate(&context)
    }

    fn calculate_command_key_score(
        &self,
        pattern: &str,
        key: &str,
        index: usize,
        frequency: u64,
        last_used_ts: u64,
    ) -> f64 {
        let is_prefix_match = key.starts_with(pattern);
        let history_weight = Self::calculate_history_weight(index);

        let context = ScoringContext::new(pattern, key)
            .with_prefix_match(is_prefix_match)
            .with_history_weight(history_weight)
            .with_history_position(index)
            .with_frequency(frequency as usize)
            .with_last_used_timestamp(last_used_ts)
            .with_source("history");

        let scorer = CompositeScorer::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(FrecencyScorer),
        ]);

        scorer.calculate(&context)
    }

    /// 计算历史权重（基于位置）
    ///
    /// 越新的命令权重越高，使用指数衰减
    fn calculate_history_weight(index: usize) -> f64 {
        // 前 100 个命令权重从 1.0 衰减到 0.1
        // 100 之后权重固定为 0.1
        if index < 100 {
            1.0 - (index as f64 / 100.0) * 0.9
        } else {
            0.1
        }
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

        let entries = provider.parse_history_content(bash_content);
        let commands: Vec<String> = entries.into_iter().map(|e| e.command).collect();

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

        let entries = provider.parse_history_content(zsh_content);
        let commands: Vec<String> = entries.iter().map(|e| e.command.clone()).collect();

        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"cd /home/user".to_string()));
        assert!(commands.contains(&"git status".to_string()));
        assert!(commands.contains(&"npm install".to_string()));
        assert_eq!(commands.len(), 4);

        // zsh: should parse timestamp
        assert!(entries.iter().any(|e| e.command == "ls -la" && e.last_used_ts.is_some()));
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

        // 测试相同输入，不同位置
        let score1 = provider.calculate_command_score("git status", "git", 0, None);
        let score2 = provider.calculate_command_score("git status", "git", 10, None);
        let score3 = provider.calculate_command_score("git status", "git", 50, None);

        // 位置越靠前分数越高（新评分系统使用历史权重+位置加分）
        assert!(
            score1 >= score2,
            "位置 0 应该 >= 位置 10: {} vs {}",
            score1,
            score2
        );
        assert!(
            score2 >= score3,
            "位置 10 应该 >= 位置 50: {} vs {}",
            score2,
            score3
        );

        // 测试匹配度：更短的输入匹配更短的命令应该得分更高
        let score_short = provider.calculate_command_score("git", "g", 0, None);
        let score_long = provider.calculate_command_score("git status", "git", 0, None);

        // 两者都是前缀匹配，分数应该都大于 0
        assert!(score_short > 0.0, "短匹配应该有分数: {}", score_short);
        assert!(score_long > 0.0, "长匹配应该有分数: {}", score_long);
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
