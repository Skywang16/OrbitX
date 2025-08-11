//! 历史命令补全提供者
//!
//! 基于用户的命令历史提供补全建议

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::cache::UnifiedCache;
use crate::utils::error::AppResult;
use anyhow::Context;
use async_trait::async_trait;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;

/// 历史命令补全提供者
pub struct HistoryProvider {
    /// 历史文件路径
    history_file: Option<PathBuf>,
    /// 最大历史记录数
    max_entries: usize,
    /// 统一缓存
    cache: Arc<UnifiedCache>,
}

impl HistoryProvider {
    /// 创建新的历史提供者
    pub fn new(cache: Arc<UnifiedCache>) -> Self {
        let mut provider = Self {
            history_file: None,
            max_entries: 1000,
            cache,
        };

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

    /// 从文件读取历史记录
    async fn read_history(&self) -> AppResult<Vec<String>> {
        let cache_key = "completion:history:all";

        // 尝试从缓存获取
        if let Some(cached_value) = self.cache.get(cache_key).await {
            if let Ok(history) = serde_json::from_value(cached_value) {
                return Ok(history);
            }
        }

        // 缓存未命中，从文件读取
        if let Some(history_file) = &self.history_file {
            if history_file.exists() {
                let content = fs::read_to_string(history_file)
                    .await
                    .context("读取历史文件失败")?;
                let history = self.parse_history_content(&content);

                // 存入缓存
                if let Ok(history_value) = serde_json::to_value(&history) {
                    let _ = self.cache.set(cache_key, history_value).await;
                }

                return Ok(history);
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

    /// 获取匹配的历史命令
    async fn get_matching_commands(&self, pattern: &str) -> AppResult<Vec<CompletionItem>> {
        let commands = self.read_history().await?;
        let mut matches = Vec::new();

        for command in commands {
            if command.starts_with(pattern) && command != pattern {
                let item = CompletionItem::new(command, CompletionType::History)
                    .with_score(70.0)
                    .with_description("历史命令".to_string())
                    .with_source("history".to_string());
                matches.push(item);
            }
        }

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

        self.get_matching_commands(&context.current_word).await
    }

    fn priority(&self) -> i32 {
        15 // 历史命令优先级最高 - 智能学习用户习惯
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
