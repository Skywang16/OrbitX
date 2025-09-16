//! 智能补全提供者

use crate::completion::context_analyzer::{
    ArgType, CompletionContext, CompletionPosition, ContextAnalyzer,
};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionItem, CompletionType};
use crate::utils::error::AppResult;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SmartCompletionProvider {
    context_analyzer: Arc<ContextAnalyzer>,
    filesystem_provider: Arc<dyn CompletionProvider>,
    system_commands_provider: Arc<dyn CompletionProvider>,
    history_provider: Arc<dyn CompletionProvider>,
}

impl SmartCompletionProvider {
    pub fn new(
        filesystem_provider: Arc<dyn CompletionProvider>,
        system_commands_provider: Arc<dyn CompletionProvider>,
        history_provider: Arc<dyn CompletionProvider>,
    ) -> Self {
        Self {
            context_analyzer: Arc::new(ContextAnalyzer::new()),
            filesystem_provider,
            system_commands_provider,
            history_provider,
        }
    }

    /// 基于上下文智能提供补全
    async fn provide_smart_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        match &context.position {
            CompletionPosition::Command => self.provide_command_completions(context).await,
            CompletionPosition::Option => self.provide_option_completions(context).await,
            CompletionPosition::OptionValue { option } => {
                self.provide_option_value_completions(context, option).await
            }
            CompletionPosition::Subcommand { parent } => {
                self.provide_subcommand_completions(context, parent).await
            }
            CompletionPosition::Argument { command, position } => {
                self.provide_argument_completions(context, command, *position)
                    .await
            }
            CompletionPosition::FilePath => self.provide_filepath_completions(context).await,
            CompletionPosition::Unknown => self.provide_fallback_completions(context).await,
        }
    }

    /// 提供命令补全
    async fn provide_command_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 优先从历史记录获取相关命令
        if let Ok(history_items) = self
            .history_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(history_items);
        }

        // 然后从系统命令获取
        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(system_items);
        }

        // 去重并按分数排序
        items = self.deduplicate_and_score(items, &context.current_word);

        Ok(items)
    }

    /// 提供选项补全
    async fn provide_option_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        if let Some(token) = context.tokens.first() {
            let command = &token.text;

            // 从命令知识库获取选项
            if let Some(meta) = self.context_analyzer.get_command_meta(command) {
                for option in &meta.options {
                    // 添加长选项
                    if let Some(long) = &option.long {
                        if long.starts_with(&context.current_word) {
                            let mut item =
                                CompletionItem::new(long.clone(), CompletionType::Option)
                                    .with_description(option.description.clone())
                                    .with_source("builtin".to_string())
                                    .with_score(90.0);

                            if option.takes_value {
                                item = item.with_display_text(format!("{} <value>", long));
                            }

                            items.push(item);
                        }
                    }

                    // 添加短选项
                    if let Some(short) = &option.short {
                        if short.starts_with(&context.current_word) {
                            let mut item =
                                CompletionItem::new(short.clone(), CompletionType::Option)
                                    .with_description(option.description.clone())
                                    .with_source("builtin".to_string())
                                    .with_score(85.0);

                            if option.takes_value {
                                item = item.with_display_text(format!("{} <value>", short));
                            }

                            items.push(item);
                        }
                    }
                }
            }

            // 从历史记录中获取该命令的常用选项
            if let Ok(history_items) = self
                .history_provider
                .provide_completions(&self.convert_context(context))
                .await
            {
                for item in history_items {
                    if item.text.starts_with('-') && item.text.starts_with(&context.current_word) {
                        let score = item.score;
                        items.push(
                            item.with_source("history".to_string())
                                .with_score(score * 0.8),
                        );
                    }
                }
            }
        }

        // 按分数排序
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(items)
    }

    /// 提供选项值补全
    async fn provide_option_value_completions(
        &self,
        context: &CompletionContext,
        option: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 根据选项类型提供补全
        if self.is_file_option(option) {
            // 文件类型选项
            if let Ok(file_items) = self
                .filesystem_provider
                .provide_completions(&self.convert_context(context))
                .await
            {
                items.extend(file_items);
            }
        } else if self.is_directory_option(option) {
            // 目录类型选项
            if let Ok(dir_items) = self
                .filesystem_provider
                .provide_completions(&self.convert_context(context))
                .await
            {
                let dir_items: Vec<_> = dir_items
                    .into_iter()
                    .filter(|item| item.completion_type == CompletionType::Directory.to_string())
                    .collect();
                items.extend(dir_items);
            }
        } else {
            // 从历史记录中查找该选项的常用值
            if let Ok(history_items) = self
                .history_provider
                .provide_completions(&self.convert_context(context))
                .await
            {
                items.extend(history_items);
            }
        }

        Ok(items)
    }

    /// 提供子命令补全
    async fn provide_subcommand_completions(
        &self,
        context: &CompletionContext,
        parent: &str,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 从命令知识库获取子命令
        if let Some(meta) = self.context_analyzer.get_command_meta(parent) {
            for subcommand in &meta.subcommands {
                if subcommand.starts_with(&context.current_word) {
                    let item = CompletionItem::new(subcommand.clone(), CompletionType::Subcommand)
                        .with_description(format!("{} 子命令", parent))
                        .with_source("builtin".to_string())
                        .with_score(95.0);
                    items.push(item);
                }
            }
        }

        // 从历史记录中获取常用的子命令组合
        if let Ok(history_items) = self
            .history_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            for item in history_items {
                // 过滤出看起来像子命令的项
                if !item.text.starts_with('-') && item.text.starts_with(&context.current_word) {
                    let score = item.score;
                    items.push(
                        item.with_source("history".to_string())
                            .with_score(score * 0.9),
                    );
                }
            }
        }

        // 按分数排序并去重
        items = self.deduplicate_and_score(items, &context.current_word);

        Ok(items)
    }

    /// 提供参数补全
    async fn provide_argument_completions(
        &self,
        context: &CompletionContext,
        command: &str,
        position: usize,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 根据命令的参数类型提供补全
        if let Some(meta) = self.context_analyzer.get_command_meta(command) {
            if let Some(arg_type) = meta.arg_types.get(position) {
                match arg_type {
                    ArgType::File | ArgType::Directory => {
                        if let Ok(file_items) = self
                            .filesystem_provider
                            .provide_completions(&self.convert_context(context))
                            .await
                        {
                            if matches!(arg_type, ArgType::Directory) {
                                let dir_items: Vec<_> = file_items
                                    .into_iter()
                                    .filter(|item| {
                                        item.completion_type
                                            == CompletionType::Directory.to_string()
                                    })
                                    .collect();
                                items.extend(dir_items);
                            } else {
                                items.extend(file_items);
                            }
                        }
                    }
                    ArgType::Enum(values) => {
                        for value in values {
                            if value.starts_with(&context.current_word) {
                                let item =
                                    CompletionItem::new(value.clone(), CompletionType::Value)
                                        .with_source("builtin".to_string())
                                        .with_score(90.0);
                                items.push(item);
                            }
                        }
                    }
                    _ => {
                        // 对于其他类型，从历史记录获取
                        if let Ok(history_items) = self
                            .history_provider
                            .provide_completions(&self.convert_context(context))
                            .await
                        {
                            items.extend(history_items);
                        }
                    }
                }
            }
        } else {
            // 没有元数据的命令，根据启发式规则
            if self.command_usually_takes_files(command) {
                if let Ok(file_items) = self
                    .filesystem_provider
                    .provide_completions(&self.convert_context(context))
                    .await
                {
                    items.extend(file_items);
                }
            } else {
                // 从历史记录获取
                if let Ok(history_items) = self
                    .history_provider
                    .provide_completions(&self.convert_context(context))
                    .await
                {
                    items.extend(history_items);
                }
            }
        }

        Ok(items)
    }

    /// 提供文件路径补全
    async fn provide_filepath_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        self.filesystem_provider
            .provide_completions(&self.convert_context(context))
            .await
    }

    /// 提供后备补全
    async fn provide_fallback_completions(
        &self,
        context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 尝试所有提供者
        if let Ok(history_items) = self
            .history_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(history_items);
        }

        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(system_items);
        }

        if let Ok(file_items) = self
            .filesystem_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(file_items);
        }

        // 限制数量并排序
        items = self.deduplicate_and_score(items, &context.current_word);
        items.truncate(20); // 限制后备补全数量

        Ok(items)
    }

    /// 转换上下文格式
    fn convert_context(
        &self,
        context: &CompletionContext,
    ) -> crate::completion::types::CompletionContext {
        crate::completion::types::CompletionContext::new(
            context.input.clone(),
            context.cursor_position,
            std::path::PathBuf::from("."), // 默认工作目录
        )
    }

    /// 去重并评分
    fn deduplicate_and_score(
        &self,
        items: Vec<CompletionItem>,
        current_word: &str,
    ) -> Vec<CompletionItem> {
        let mut seen: std::collections::HashMap<String, CompletionItem> =
            std::collections::HashMap::new();
        let mut deduplicated = Vec::new();

        for item in items {
            if let Some(existing) = seen.get(&item.text) {
                // 保留分数更高的
                if item.score > existing.score {
                    seen.insert(item.text.clone(), item.clone());
                }
            } else {
                seen.insert(item.text.clone(), item.clone());
            }
        }

        for (_, item) in seen {
            deduplicated.push(item);
        }

        // 重新计算分数，考虑前缀匹配
        for item in &mut deduplicated {
            if item.text.starts_with(current_word) {
                item.score += 10.0; // 前缀匹配加分

                // 完全匹配但不是完全相同的情况下再加分
                if item.text.len() > current_word.len() {
                    let match_ratio = current_word.len() as f64 / item.text.len() as f64;
                    item.score += match_ratio * 20.0;
                }
            }
        }

        // 按分数排序
        deduplicated.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        deduplicated
    }

    /// 检查是否是文件类型选项
    fn is_file_option(&self, option: &str) -> bool {
        matches!(
            option,
            "--file" | "--input" | "--output" | "--config" | "--script" | "-f" | "-i" | "-o" | "-c"
        )
    }

    /// 检查是否是目录类型选项
    fn is_directory_option(&self, option: &str) -> bool {
        matches!(
            option,
            "--directory" | "--dir" | "--path" | "--workdir" | "-d" | "-p"
        )
    }

    /// 检查命令是否通常接受文件参数
    fn command_usually_takes_files(&self, command: &str) -> bool {
        matches!(
            command,
            "cat"
                | "less"
                | "more"
                | "head"
                | "tail"
                | "grep"
                | "awk"
                | "sed"
                | "cp"
                | "mv"
                | "rm"
                | "chmod"
                | "chown"
                | "file"
                | "wc"
                | "sort"
                | "uniq"
                | "cut"
                | "tr"
                | "vim"
                | "nano"
                | "emacs"
                | "code"
        )
    }
}

#[async_trait]
impl CompletionProvider for SmartCompletionProvider {
    fn name(&self) -> &'static str {
        "smart"
    }

    fn should_provide(&self, _context: &crate::completion::types::CompletionContext) -> bool {
        true // 智能提供者总是可以提供补全
    }

    async fn provide_completions(
        &self,
        context: &crate::completion::types::CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        // 使用上下文分析器分析输入
        let smart_context = self
            .context_analyzer
            .analyze(&context.input, context.cursor_position);

        // 基于分析结果提供智能补全
        self.provide_smart_completions(&smart_context).await
    }

    fn priority(&self) -> i32 {
        100 // 最高优先级
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
