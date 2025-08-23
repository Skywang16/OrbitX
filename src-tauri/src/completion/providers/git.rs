//! Git命令增强补全提供者
//!
//! 为git命令提供智能补全，包括：
//! - 分支名称补全
//! - 远程仓库补全
//! - 标签补全
//! - 文件状态补全

use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use anyhow::Context;
use async_trait::async_trait;

use std::path::Path;
use tokio::process::Command as AsyncCommand;

/// Git补全提供者
pub struct GitCompletionProvider {
    /// 使用统一缓存
    cache: crate::storage::cache::UnifiedCache,
}

impl GitCompletionProvider {
    /// 创建新的Git补全提供者
    pub fn new() -> Self {
        Self {
            cache: crate::storage::cache::UnifiedCache::new(),
        }
    }

    /// 检查是否在git仓库中
    async fn is_git_repository(&self, working_directory: &Path) -> AppResult<bool> {
        let path_str = working_directory.to_string_lossy().to_string();
        let cache_key = format!("git_repo:{}", path_str);

        // 检查缓存
        if let Some(cached_result) = self.cache.get(&cache_key).await {
            if let Some(is_repo) = cached_result.as_bool() {
                return Ok(is_repo);
            }
        }

        // 执行git命令
        let output = AsyncCommand::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(working_directory)
            .output()
            .await
            .with_context(|| "检查git仓库失败")?;

        let is_repo = output.status.success();

        // 缓存结果
        let _ = self
            .cache
            .set(&cache_key, serde_json::Value::Bool(is_repo))
            .await;

        Ok(is_repo)
    }

    /// 解析git命令
    fn parse_git_command(&self, context: &CompletionContext) -> Option<(String, Vec<String>)> {
        let parts: Vec<&str> = context.input.split_whitespace().collect();
        if parts.is_empty() || parts[0] != "git" {
            return None;
        }

        if parts.len() == 1 {
            // 只有"git"，补全子命令
            return Some(("".to_string(), vec![]));
        }

        let subcommand = parts[1].to_string();
        let args = parts[2..].iter().map(|s| s.to_string()).collect();
        Some((subcommand, args))
    }

    /// 获取分支补全
    async fn get_branch_completions(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let output = AsyncCommand::new("git")
            .args(["branch", "--all", "--format=%(refname:short)"])
            .current_dir(working_directory)
            .output()
            .await
            .with_context(|| "获取分支列表失败")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let branches_output = String::from_utf8_lossy(&output.stdout);
        let mut completions = Vec::new();

        for line in branches_output.lines() {
            let branch = line.trim();
            if branch.is_empty() || branch.starts_with("origin/HEAD") {
                continue;
            }

            // 简单的前缀匹配
            if !query.is_empty() && !branch.to_lowercase().starts_with(&query.to_lowercase()) {
                continue;
            }

            let (completion_type, description, score) = if branch.starts_with("origin/") {
                (CompletionType::Value, format!("远程分支: {}", branch), 8.0)
            } else {
                (CompletionType::Value, format!("本地分支: {}", branch), 10.0)
            };

            let mut item = CompletionItem::new(branch.to_string(), completion_type)
                .with_description(description)
                .with_score(score)
                .with_source("git".to_string());

            // 添加元数据
            item = item.with_metadata("type".to_string(), "branch".to_string());
            if branch.starts_with("origin/") {
                item = item.with_metadata("remote".to_string(), "true".to_string());
            }

            completions.push(item);
        }

        Ok(completions)
    }

    /// 获取git子命令补全
    fn get_subcommand_completions(&self, query: &str) -> Vec<CompletionItem> {
        let subcommands = vec![
            ("add", "添加文件到暂存区"),
            ("commit", "提交更改"),
            ("push", "推送到远程仓库"),
            ("pull", "从远程仓库拉取"),
            ("checkout", "切换分支或恢复文件"),
            ("branch", "分支管理"),
            ("merge", "合并分支"),
            ("status", "查看状态"),
            ("log", "查看提交历史"),
            ("diff", "查看差异"),
            ("reset", "重置更改"),
            ("tag", "标签管理"),
            ("remote", "远程仓库管理"),
            ("clone", "克隆仓库"),
            ("init", "初始化仓库"),
        ];

        let mut completions = Vec::new();
        for (cmd, desc) in subcommands {
            if query.is_empty() || cmd.starts_with(query) {
                let score = if cmd.starts_with(query) { 10.0 } else { 5.0 };

                let item = CompletionItem::new(cmd.to_string(), CompletionType::Subcommand)
                    .with_description(desc.to_string())
                    .with_score(score)
                    .with_source("git".to_string())
                    .with_metadata("type".to_string(), "subcommand".to_string());

                completions.push(item);
            }
        }

        completions
    }

    /// 获取文件状态补全（用于git add）
    async fn get_file_status_completions(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let output = AsyncCommand::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_directory)
            .output()
            .await
            .with_context(|| "获取文件状态失败")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let mut completions = Vec::new();

        for line in status_output.lines() {
            if line.len() >= 3 {
                let status = &line[0..2];
                let filename = &line[3..];

                // 简单的前缀匹配
                if !query.is_empty() && !filename.to_lowercase().starts_with(&query.to_lowercase())
                {
                    continue;
                }

                let (description, score) = match status {
                    "??" => ("未跟踪文件", 10.0),
                    " M" => ("已修改文件", 15.0),
                    "M " => ("已暂存文件", 5.0),
                    " D" => ("已删除文件", 12.0),
                    "A " => ("新增文件", 8.0),
                    _ => ("其他状态文件", 6.0),
                };

                let item = CompletionItem::new(filename.to_string(), CompletionType::File)
                    .with_description(format!("{}: {}", description, filename))
                    .with_score(score)
                    .with_source("git".to_string())
                    .with_metadata("type".to_string(), "file".to_string())
                    .with_metadata("status".to_string(), status.to_string());

                completions.push(item);
            }
        }

        Ok(completions)
    }
}

#[async_trait]
impl CompletionProvider for GitCompletionProvider {
    fn name(&self) -> &'static str {
        "git"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // 检查是否是git命令
        context.input.trim_start().starts_with("git ")
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        // 检查是否在git仓库中
        if !self.is_git_repository(&context.working_directory).await? {
            return Ok(vec![]);
        }

        // 解析git命令
        let (subcommand, _args) = match self.parse_git_command(context) {
            Some(parsed) => parsed,
            None => return Ok(vec![]),
        };

        // 如果没有子命令，提供子命令补全
        if subcommand.is_empty() {
            return Ok(self.get_subcommand_completions(&context.current_word));
        }

        // 根据子命令提供相应的补全
        match subcommand.as_str() {
            "checkout" | "co" | "merge" | "branch" => {
                self.get_branch_completions(&context.working_directory, &context.current_word)
                    .await
            }
            "add" => {
                self.get_file_status_completions(&context.working_directory, &context.current_word)
                    .await
            }
            _ => Ok(vec![]),
        }
    }

    fn priority(&self) -> i32 {
        15 // 高优先级，因为这是专门的git补全
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for GitCompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}
