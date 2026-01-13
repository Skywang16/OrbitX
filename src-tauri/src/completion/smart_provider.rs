//! 智能补全提供者

use crate::completion::command_line::extract_command_key;
use crate::completion::context_analyzer::{
    ArgType, CompletionPosition, ContextAnalysis, ContextAnalyzer,
};
use crate::completion::error::CompletionProviderResult;
use crate::completion::metadata::CommandRegistry;
use crate::completion::prediction::CommandPredictor;
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::repositories::CompletionModelRepo;
use crate::storage::DatabaseManager;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SmartCompletionProvider {
    context_analyzer: Arc<ContextAnalyzer>,
    filesystem_provider: Arc<dyn CompletionProvider>,
    system_commands_provider: Arc<dyn CompletionProvider>,
    history_provider: Arc<dyn CompletionProvider>,
    context_aware_provider: Option<Arc<dyn CompletionProvider>>,
    predictor: Option<CommandPredictor>,
    database: Arc<DatabaseManager>,
}

impl SmartCompletionProvider {
    pub fn new(
        filesystem_provider: Arc<dyn CompletionProvider>,
        system_commands_provider: Arc<dyn CompletionProvider>,
        history_provider: Arc<dyn CompletionProvider>,
        database: Arc<DatabaseManager>,
    ) -> Self {
        // 获取当前工作目录，初始化预测器
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let predictor = Some(CommandPredictor::new(current_dir));

        Self {
            context_analyzer: Arc::new(ContextAnalyzer::new()),
            filesystem_provider,
            system_commands_provider,
            history_provider,
            context_aware_provider: None,
            predictor,
            database,
        }
    }

    /// 设置上下文感知提供者（用于实体增强）
    pub fn with_context_aware(mut self, provider: Arc<dyn CompletionProvider>) -> Self {
        self.context_aware_provider = Some(provider);
        self
    }

    /// 基于上下文智能提供补全
    async fn provide_smart_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        match &analysis.position {
            CompletionPosition::Command => self.provide_command_completions(context).await,
            CompletionPosition::Option => self.provide_option_completions(context, analysis).await,
            CompletionPosition::OptionValue { option } => {
                self.provide_option_value_completions(context, analysis, option)
                    .await
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // `cd` without a trailing space: still offer directory completions by inserting a leading space.
        if context.input.trim() == "cd"
            && context.cursor_position == context.input.len()
            && !context.input.chars().any(|c| c.is_whitespace())
        {
            let ctx = crate::completion::types::CompletionContext::new(
                "cd ".to_string(),
                context.cursor_position + 1,
                context.working_directory.clone(),
            );
            if let Ok(mut dir_items) = self.filesystem_provider.provide_completions(&ctx).await {
                for item in &mut dir_items {
                    item.text = format!(" {}", item.text);
                    item.score = item.score.max(90.0);
                    item.source = Some("smart".to_string());
                }
                items.extend(dir_items);
            }
        }

        // 步骤1: 下一条命令预测（学习模型优先，静态表兜底）
        items.extend(self.predict_next_command_items(context).await);

        // 步骤2: 从历史记录获取相关命令
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
            items.extend(history_items);
        }

        // 步骤3: 从系统命令获取
        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(context)
            .await
        {
            items.extend(system_items);
        }

        // 步骤4: 去重、规范化分数、排序
        items = self.deduplicate_and_sort(items);

        Ok(items)
    }

    /// 提供选项补全
    async fn provide_option_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        if let Some(token) = analysis.tokens.first() {
            let command = token.text(&context.input);

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
                                item = item.with_display_text(format!("{long} <value>"));
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
                                item = item.with_display_text(format!("{short} <value>"));
                            }

                            items.push(item);
                        }
                    }
                }
            }

            // 从历史记录中获取该命令的常用选项
            if let Ok(history_items) = self.history_provider.provide_completions(context).await {
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

        // 按分数排序（使用 CompletionItem 的 Ord 实现）
        items.sort_unstable();

        Ok(items)
    }

    /// 提供选项值补全
    async fn provide_option_value_completions(
        &self,
        context: &CompletionContext,
        analysis: &ContextAnalysis,
        option: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 获取命令名（用于查找元数据）
        let command = analysis
            .tokens
            .first()
            .map(|t| t.text(&context.input))
            .unwrap_or("");

        // 根据选项类型提供补全
        if self.is_file_option(command, option) {
            // 文件类型选项
            if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await {
                items.extend(file_items);
            }
        } else if self.is_directory_option(command, option) {
            // 目录类型选项
            if let Ok(dir_items) = self.filesystem_provider.provide_completions(context).await {
                let dir_items: Vec<_> = dir_items
                    .into_iter()
                    .filter(|item| item.completion_type == CompletionType::Directory.to_string())
                    .collect();
                items.extend(dir_items);
            }
        } else {
            // 从历史记录中查找该选项的常用值
            if let Ok(history_items) = self.history_provider.provide_completions(context).await {
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 从命令知识库获取子命令
        if let Some(meta) = self.context_analyzer.get_command_meta(parent) {
            for subcommand in &meta.subcommands {
                if subcommand.starts_with(&context.current_word) {
                    let item = CompletionItem::new(subcommand.clone(), CompletionType::Subcommand)
                        .with_description(format!("{parent} 子命令"))
                        .with_source("builtin".to_string())
                        .with_score(95.0);
                    items.push(item);
                }
            }
        }

        // 从历史记录中获取常用的子命令组合
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
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
        items = self.deduplicate_and_sort(items);

        Ok(items)
    }

    /// 提供参数补全
    async fn provide_argument_completions(
        &self,
        context: &CompletionContext,
        command: &str,
        position: usize,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 根据命令的参数类型提供补全
        if let Some(meta) = self.context_analyzer.get_command_meta(command) {
            if let Some(arg_type) = meta.arg_types.get(position) {
                match arg_type {
                    ArgType::File | ArgType::Directory => {
                        if let Ok(file_items) =
                            self.filesystem_provider.provide_completions(context).await
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
                        if let Ok(history_items) =
                            self.history_provider.provide_completions(context).await
                        {
                            items.extend(history_items);
                        }
                    }
                }
            }
        } else {
            // 没有元数据的命令，根据启发式规则
            if self.command_usually_takes_files(command) {
                if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await
                {
                    items.extend(file_items);
                }
            } else {
                // 从历史记录获取
                if let Ok(history_items) = self.history_provider.provide_completions(context).await
                {
                    items.extend(history_items);
                }
            }
        }

        // 增强补全：从 OutputAnalyzer 提取的实体添加相关补全
        items.extend(self.enhance_with_entities(command, context).await);

        Ok(items)
    }

    /// 使用 OutputAnalyzer 提取的实体增强补全
    async fn enhance_with_entities(
        &self,
        command: &str,
        _context: &CompletionContext,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // 如果没有上下文提供者，跳过
        if self.context_aware_provider.is_none() {
            return items;
        }

        use crate::completion::output_analyzer::OutputAnalyzer;

        let analyzer = OutputAnalyzer::global();
        let provider = analyzer.context_provider();

        match command {
            "kill" | "killall" => {
                // 为 kill 命令添加最近的 PID
                for pid in provider.get_recent_pids() {
                    let item = CompletionItem::new(pid, CompletionType::Value)
                        .with_score(85.0)
                        .with_description("最近的进程ID".to_string())
                        .with_source("context".to_string());
                    items.push(item);
                }
            }
            "lsof" => {
                // 为 lsof 命令添加最近的端口
                for port in provider.get_recent_ports() {
                    let item = CompletionItem::new(port, CompletionType::Value)
                        .with_score(85.0)
                        .with_description("最近使用的端口".to_string())
                        .with_source("context".to_string());
                    items.push(item);
                }
            }
            "cd" => {
                // 为 cd 命令添加最近访问的目录
                for path in provider.get_recent_paths() {
                    // 只添加目录
                    if std::path::Path::new(&path).is_dir() {
                        let item = CompletionItem::new(path, CompletionType::Directory)
                            .with_score(80.0)
                            .with_description("最近访问的目录".to_string())
                            .with_source("context".to_string());
                        items.push(item);
                    }
                }
            }
            _ => {}
        }

        items
    }

    /// 提供文件路径补全
    async fn provide_filepath_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        self.filesystem_provider.provide_completions(context).await
    }

    /// 提供后备补全
    async fn provide_fallback_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 尝试所有提供者
        if let Ok(history_items) = self.history_provider.provide_completions(context).await {
            items.extend(history_items);
        }

        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(context)
            .await
        {
            items.extend(system_items);
        }

        if let Ok(file_items) = self.filesystem_provider.provide_completions(context).await {
            items.extend(file_items);
        }

        // 限制数量并排序
        items = self.deduplicate_and_sort(items);
        items.truncate(20); // 限制后备补全数量

        Ok(items)
    }

    async fn predict_next_command_items(&self, context: &CompletionContext) -> Vec<CompletionItem> {
        use crate::completion::output_analyzer::OutputAnalyzer;

        let Some(ref predictor) = self.predictor else {
            return Vec::new();
        };

        let analyzer = OutputAnalyzer::global();
        let (last_cmd, last_output) = {
            let provider = analyzer.context_provider();
            let Some(last) = provider.get_last_command() else {
                return Vec::new();
            };

            last
        };

        // 1) 学习模型：prev_key -> next_key topK
        let mut predictions = self
            .learned_next_commands(&last_cmd, &context.current_word)
            .await
            .into_iter()
            .map(|(suggested, confidence)| {
                let mut pred = predictor.build_prediction_for_suggestion(
                    &suggested,
                    &last_cmd,
                    Some(&last_output),
                );
                pred.confidence = pred.confidence.max(confidence).min(100.0);
                pred
            })
            .collect::<Vec<_>>();

        // 2) 兜底：静态工作流表
        if predictions.is_empty() {
            predictions = predictor.predict_next_commands(
                &last_cmd,
                Some(&last_output),
                &context.current_word,
            );
        }

        predictions
            .into_iter()
            .map(|p| p.to_completion_item())
            .collect()
    }

    async fn learned_next_commands(
        &self,
        last_command: &str,
        input_prefix: &str,
    ) -> Vec<(String, f64)> {
        let Some(prev_key) = extract_command_key(last_command) else {
            return Vec::new();
        };

        let repo = CompletionModelRepo::new(&self.database);
        let Ok(Some(prev_id)) = repo.find_key_id(&prev_key.key).await else {
            return Vec::new();
        };

        let Ok(rows) = repo.top_next_keys(prev_id, 20).await else {
            return Vec::new();
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        rows.into_iter()
            .filter(|(key, _, _, _)| input_prefix.is_empty() || key.starts_with(input_prefix))
            .map(|(key, count, success_count, last_used_ts)| {
                let confidence = transition_confidence(
                    now,
                    count as u64,
                    success_count as u64,
                    last_used_ts as u64,
                );
                (key, confidence)
            })
            .collect()
    }

    fn deduplicate_and_sort(&self, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        let mut seen: std::collections::HashMap<String, CompletionItem> =
            std::collections::HashMap::new();

        for mut item in items {
            item.score = item.score.clamp(0.0, 100.0);
            seen.entry(item.text.clone())
                .and_modify(|existing| {
                    if item.score > existing.score {
                        *existing = item.clone();
                    }
                })
                .or_insert(item);
        }

        let mut deduped: Vec<CompletionItem> = seen.into_values().collect();
        deduped.sort_unstable();
        deduped
    }

    /// 检查是否是文件类型选项（使用命令注册表）
    fn is_file_option(&self, command: &str, option: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.is_file_option(command, option)
    }

    /// 检查是否是目录类型选项（使用命令注册表）
    fn is_directory_option(&self, command: &str, option: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.is_directory_option(command, option)
    }

    /// 检查命令是否通常接受文件参数（使用命令注册表）
    fn command_usually_takes_files(&self, command: &str) -> bool {
        let registry = CommandRegistry::global();
        registry.accepts_files(command)
    }
}

fn transition_confidence(now_ts: u64, count: u64, success_count: u64, last_used_ts: u64) -> f64 {
    if count == 0 {
        return 0.0;
    }

    let success_rate = (success_count as f64 / count as f64).clamp(0.0, 1.0);
    let seconds_ago = now_ts.saturating_sub(last_used_ts);
    let recency = match seconds_ago {
        0..=3600 => 1.0,
        3601..=86400 => 0.8,
        86401..=604800 => 0.6,
        604801..=2592000 => 0.4,
        _ => 0.2,
    };

    let count_factor = ((count as f64).ln_1p() / 4.0).min(1.0); // ln(54)~4
    (count_factor * 60.0 + recency * 25.0 + success_rate * 15.0).min(100.0)
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let analysis = self
            .context_analyzer
            .analyze(&context.input, context.cursor_position);
        self.provide_smart_completions(context, &analysis).await
    }

    fn priority(&self) -> i32 {
        100 // 最高优先级
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
