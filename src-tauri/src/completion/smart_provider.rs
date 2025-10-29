//! 智能补全提供者

use crate::completion::context_analyzer::{
    ArgType, CompletionContext, CompletionPosition, ContextAnalyzer,
};
use crate::completion::error::CompletionProviderResult;
use crate::completion::metadata::CommandRegistry;
use crate::completion::prediction::CommandPredictor;
use crate::completion::providers::CompletionProvider;
use crate::completion::scoring::{BaseScorer, ScoreCalculator, ScoringContext};
use crate::completion::types::{CompletionItem, CompletionType};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SmartCompletionProvider {
    context_analyzer: Arc<ContextAnalyzer>,
    filesystem_provider: Arc<dyn CompletionProvider>,
    system_commands_provider: Arc<dyn CompletionProvider>,
    history_provider: Arc<dyn CompletionProvider>,
    context_aware_provider: Option<Arc<dyn CompletionProvider>>,
    predictor: Option<CommandPredictor>,
}

impl SmartCompletionProvider {
    pub fn new(
        filesystem_provider: Arc<dyn CompletionProvider>,
        system_commands_provider: Arc<dyn CompletionProvider>,
        history_provider: Arc<dyn CompletionProvider>,
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 步骤1: 智能预测 - 根据上一条命令预测下一条
        if let Some(ref predictor) = self.predictor {
            use crate::completion::output_analyzer::OutputAnalyzer;
            let analyzer = OutputAnalyzer::global();
            if let Ok(provider) = analyzer.get_context_provider().lock() {
                if let Some((last_cmd, last_output)) = provider.get_last_command() {
                    let predictions = predictor.predict_next_commands(
                        &last_cmd,
                        Some(&last_output),
                        &context.current_word,
                    );

                    // 转换为补全项，预测命令得分 90-95
                    for pred in predictions {
                        items.push(pred.to_completion_item());
                    }
                }
            }
        }

        // 步骤2: 从历史记录获取相关命令
        if let Ok(history_items) = self
            .history_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(history_items);
        }

        // 步骤3: 从系统命令获取
        if let Ok(system_items) = self
            .system_commands_provider
            .provide_completions(&self.convert_context(context))
            .await
        {
            items.extend(system_items);
        }

        // 步骤4: 上下文加分 - 根据工作目录特征加分
        if let Some(ref predictor) = self.predictor {
            let context_boost = predictor.calculate_context_boost(&context.current_word);
            if context_boost > 0.0 {
                // 为匹配前缀的命令加分
                for item in &mut items {
                    if item.text.starts_with(&context.current_word)
                        || context.current_word.is_empty()
                    {
                        item.score += context_boost;
                    }
                }
            }
        }

        // 步骤5: 去重并按分数排序
        items = self.deduplicate_and_score(items, &context.current_word);

        Ok(items)
    }

    /// 提供选项补全
    async fn provide_option_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
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

        // 按分数排序（使用 CompletionItem 的 Ord 实现）
        items.sort_unstable();

        Ok(items)
    }

    /// 提供选项值补全
    async fn provide_option_value_completions(
        &self,
        context: &CompletionContext,
        option: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 获取命令名（用于查找元数据）
        let command = context
            .tokens
            .first()
            .map(|t| t.text.as_str())
            .unwrap_or("");

        // 根据选项类型提供补全
        if self.is_file_option(command, option) {
            // 文件类型选项
            if let Ok(file_items) = self
                .filesystem_provider
                .provide_completions(&self.convert_context(context))
                .await
            {
                items.extend(file_items);
            }
        } else if self.is_directory_option(command, option) {
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
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
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
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
        let provider_mutex = analyzer.get_context_provider();

        match command {
            "kill" | "killall" => {
                // 为 kill 命令添加最近的 PID
                if let Ok(provider) = provider_mutex.lock() {
                    let pids = provider.get_recent_pids();
                    for pid in pids {
                        let item = CompletionItem::new(pid, CompletionType::Value)
                            .with_score(85.0)
                            .with_description("最近的进程ID".to_string())
                            .with_source("context".to_string());
                        items.push(item);
                    }
                }
            }
            "lsof" => {
                // 为 lsof 命令添加最近的端口
                if let Ok(provider) = provider_mutex.lock() {
                    let ports = provider.get_recent_ports();
                    for port in ports {
                        let item = CompletionItem::new(port, CompletionType::Value)
                            .with_score(85.0)
                            .with_description("最近使用的端口".to_string())
                            .with_source("context".to_string());
                        items.push(item);
                    }
                }
            }
            "cd" => {
                // 为 cd 命令添加最近访问的目录
                if let Ok(provider) = provider_mutex.lock() {
                    let paths = provider.get_recent_paths();
                    for path in paths {
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
        self.filesystem_provider
            .provide_completions(&self.convert_context(context))
            .await
    }

    /// 提供后备补全
    async fn provide_fallback_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
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

    /// 去重并重新评分（使用统一评分系统）
    fn deduplicate_and_score(
        &self,
        items: Vec<CompletionItem>,
        current_word: &str,
    ) -> Vec<CompletionItem> {
        let mut seen: std::collections::HashMap<String, CompletionItem> =
            std::collections::HashMap::new();

        // 去重：保留每个文本的最高分项
        for item in items {
            seen.entry(item.text.clone())
                .and_modify(|existing| {
                    if item.score > existing.score {
                        *existing = item.clone();
                    }
                })
                .or_insert(item);
        }

        let mut deduplicated: Vec<CompletionItem> = seen.into_values().collect();

        // 重新评分：使用统一评分系统
        let scorer = BaseScorer;
        for item in &mut deduplicated {
            let is_prefix_match = item.text.starts_with(current_word);
            let context = ScoringContext::new(current_word, &item.text)
                .with_prefix_match(is_prefix_match)
                .with_source("smart");

            // 保留原有分数，只加上前缀匹配加分
            item.score += scorer.calculate(&context);
        }

        // 按分数排序（使用 CompletionItem 的 Ord 实现）
        deduplicated.sort_unstable();

        deduplicated
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
