//! 历史命令补全提供者
//!
//! 基于用户的命令历史提供补全建议

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use async_trait::async_trait;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs;

/// 历史记录项
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// 命令文本
    pub command: String,
    /// 使用次数
    pub frequency: u32,
    /// 最后使用时间（Unix时间戳）
    pub last_used: u64,
}

impl HistoryEntry {
    /// 创建新的历史记录项
    pub fn new(command: String) -> Self {
        Self {
            command,
            frequency: 1,
            last_used: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// 更新使用记录
    pub fn update_usage(&mut self) {
        self.frequency += 1;
        self.last_used = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 计算权重分数（结合频率和时间）
    pub fn calculate_score(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let time_diff = now.saturating_sub(self.last_used) as f64;
        let time_factor = 1.0 / (1.0 + time_diff / 86400.0); // 时间衰减因子（以天为单位）
        let frequency_factor = (self.frequency as f64).ln() + 1.0; // 频率因子

        frequency_factor * time_factor
    }
}

/// 历史命令补全提供者
pub struct HistoryProvider {
    /// 历史记录存储
    history: Arc<RwLock<HashMap<String, HistoryEntry>>>,
    /// 历史文件路径
    history_file: Option<PathBuf>,
    /// 最大历史记录数
    max_entries: usize,
}

impl HistoryProvider {
    /// 创建新的历史提供者
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
            history_file: None,
            max_entries: 1000,
        }
    }

    /// 设置历史文件路径
    pub fn with_history_file(mut self, path: PathBuf) -> Self {
        self.history_file = Some(path);
        self
    }

    /// 设置最大历史记录数
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// 从文件加载历史记录
    pub async fn load_history(&self) -> AppResult<()> {
        if let Some(history_file) = &self.history_file {
            if history_file.exists() {
                let content = fs::read_to_string(history_file)
                    .await
                    .context("读取历史文件失败")?;

                let mut history = self
                    .history
                    .write()
                    .map_err(|_| anyhow!("提供者错误: 获取历史记录写锁失败"))?;

                // 清空现有历史记录，重新加载
                history.clear();

                let commands = self.parse_history_content(&content);

                for command in commands {
                    let entry = history
                        .entry(command.clone())
                        .or_insert_with(|| HistoryEntry::new(command));
                    entry.update_usage();
                }
            } else {
                // 历史文件不存在，跳过加载
            }
        } else {
            // 未设置历史文件路径，跳过加载
        }

        Ok(())
    }

    /// 解析历史文件内容，支持不同的shell格式
    fn parse_history_content(&self, content: &str) -> Vec<String> {
        let mut commands = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 处理zsh历史格式：: 时间戳:0;命令
            if line.starts_with(": ") && line.contains(';') {
                if let Some(cmd_start) = line.find(';') {
                    let command = line[cmd_start + 1..].trim();
                    if !command.is_empty() {
                        commands.push(command.to_string());
                    }
                }
            } else {
                // bash格式：直接是命令
                commands.push(line.to_string());
            }
        }

        // 去重并保持最近的命令
        let mut unique_commands = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 从后往前遍历，保持最新的命令
        for command in commands.into_iter().rev() {
            if seen.insert(command.clone()) {
                unique_commands.push(command);
            }
        }

        unique_commands.reverse();
        unique_commands
    }

    /// 保存历史记录到文件
    pub async fn save_history(&self) -> AppResult<()> {
        if let Some(history_file) = &self.history_file {
            let content = {
                let history = self
                    .history
                    .read()
                    .map_err(|_| anyhow!("提供者错误: 获取历史记录读锁失败"))?;

                let mut entries: Vec<_> = history.values().collect();
                entries.sort_by(|a, b| b.last_used.cmp(&a.last_used));

                entries
                    .iter()
                    .take(self.max_entries)
                    .map(|entry| entry.command.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            fs::write(history_file, content)
                .await
                .context("写入历史文件失败")?;
        }

        Ok(())
    }

    /// 添加命令到历史记录
    pub fn add_command(&self, command: String) -> AppResult<()> {
        let command = command.trim().to_string();
        if command.is_empty() {
            return Ok(());
        }

        let mut history = self
            .history
            .write()
            .map_err(|_| anyhow!("获取历史记录写锁失败"))?;

        let entry = history
            .entry(command.clone())
            .or_insert_with(|| HistoryEntry::new(command));
        entry.update_usage();

        // 如果历史记录过多，清理最旧的记录
        if history.len() > self.max_entries {
            let mut entries: Vec<_> = history
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries.sort_by(|a, b| a.1.last_used.cmp(&b.1.last_used));

            // 移除最旧的记录
            let to_remove = entries.len() - self.max_entries;
            for (key, _) in entries.iter().take(to_remove) {
                history.remove(key);
            }
        }

        Ok(())
    }

    /// 确保历史记录已加载（每次都重新加载以获取最新历史）
    async fn ensure_initialized(&self) -> AppResult<()> {
        // 每次都重新加载历史文件以获取最新的命令
        self.load_history().await
    }

    /// 获取匹配的历史命令
    fn get_matching_commands(&self, pattern: &str) -> AppResult<Vec<CompletionItem>> {
        let history = self
            .history
            .read()
            .map_err(|_| anyhow!("提供者错误: 获取历史记录读锁失败"))?;

        let mut matches = Vec::new();

        for entry in history.values() {
            // 只匹配以输入开头的命令，但排除完全相同的（没有补全价值）
            if entry.command.starts_with(pattern) && entry.command != pattern {
                let base_score = entry.calculate_score();

                // 计算标准化分数（0-100范围） - 现在都是前缀匹配，因为排除了完全相同的
                let prefix_bonus = 10.0; // 前缀匹配奖励

                // 将base_score标准化到合理范围，历史记录基础分数较高
                let normalized_score = 60.0 + (base_score * 5.0).min(20.0) + prefix_bonus;
                let final_score = normalized_score.min(100.0);

                let item = CompletionItem::new(entry.command.clone(), CompletionType::History)
                    .with_score(final_score)
                    .with_description(format!("使用 {} 次", entry.frequency))
                    .with_source("history".to_string())
                    .with_metadata("frequency".to_string(), entry.frequency.to_string())
                    .with_metadata("last_used".to_string(), entry.last_used.to_string());

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
}

#[async_trait]
impl CompletionProvider for HistoryProvider {
    fn name(&self) -> &'static str {
        "history"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // 如果输入不为空且不是以路径分隔符开头，可能是在寻找历史命令
        !context.current_word.is_empty()
            && !context.current_word.starts_with('/')
            && !context.current_word.starts_with('\\')
            && !context.current_word.starts_with('.')
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        if context.current_word.is_empty() {
            return Ok(Vec::new());
        }

        // 确保历史记录已加载
        self.ensure_initialized().await?;

        self.get_matching_commands(&context.current_word)
    }

    fn priority(&self) -> i32 {
        15 // 历史命令优先级最高 - 智能学习用户习惯
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for HistoryProvider {
    fn default() -> Self {
        let mut provider = Self::new();

        // 尝试设置默认的历史文件路径
        if let Some(home_dir) = dirs::home_dir() {
            let bash_history = home_dir.join(".bash_history");
            let zsh_history = home_dir.join(".zsh_history");

            if bash_history.exists() {
                provider = provider.with_history_file(bash_history);
            } else if zsh_history.exists() {
                provider = provider.with_history_file(zsh_history);
            }
        }

        provider
    }
}
