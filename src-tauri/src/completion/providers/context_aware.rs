//! 上下文感知补全提供者
//!
//! 基于命令执行历史和输出结果提供智能补全建议

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

/// 命令输出记录
#[derive(Debug, Clone)]
pub struct CommandOutputRecord {
    /// 命令文本
    pub command: String,
    /// 命令输出
    pub output: String,
    /// 执行时间戳
    pub timestamp: u64,
    /// 工作目录
    pub working_directory: String,
    /// 提取的实体（如PID、端口等）
    pub extracted_entities: HashMap<String, Vec<String>>,
}

/// 上下文感知补全提供者
pub struct ContextAwareProvider {
    /// 命令输出历史
    command_history: Arc<RwLock<Vec<CommandOutputRecord>>>,
    /// 最大历史记录数
    max_history: usize,
}

impl ContextAwareProvider {
    /// 创建新的上下文感知提供者
    pub fn new() -> Self {
        Self {
            command_history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// 获取最近的 PID 列表（公共方法供外部使用）
    pub fn get_recent_pids(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut pids = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(record_pids) = record.extracted_entities.get("pid") {
                pids.extend(record_pids.clone());
            }
        }

        // 去重并限制数量
        pids.sort();
        pids.dedup();
        pids.truncate(10);
        pids
    }

    /// 获取最近的端口列表（公共方法供外部使用）
    pub fn get_recent_ports(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut ports = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(record_ports) = record.extracted_entities.get("port") {
                ports.extend(record_ports.clone());
            }
        }

        ports.sort();
        ports.dedup();
        ports.truncate(10);
        ports
    }

    /// 获取最近访问的路径列表（公共方法供外部使用）
    pub fn get_recent_paths(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut paths = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(file_paths) = record.extracted_entities.get("file_path") {
                paths.extend(file_paths.clone());
            }
            if let Some(dir_paths) = record.extracted_entities.get("directory_path") {
                paths.extend(dir_paths.clone());
            }
        }

        paths.sort();
        paths.dedup();
        paths.truncate(10);
        paths
    }

    /// 获取最后一条命令及其输出（供预测器使用）
    pub fn get_last_command(&self) -> Option<(String, String)> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return None,
        };

        history
            .last()
            .map(|record| (record.command.clone(), record.output.clone()))
    }

    /// 记录命令输出
    pub fn record_command_output(
        &self,
        command: String,
        output: String,
        working_directory: String,
    ) -> CompletionProviderResult<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 使用智能提取器提取实体（可失败，失败时不影响记录）
        use crate::completion::smart_extractor::SmartExtractor;
        let extractor = SmartExtractor::global();
        let extracted_entities = match extractor.extract_entities(&command, &output) {
            Ok(results) => {
                let mut map = HashMap::new();
                for result in results {
                    map.entry(result.entity_type)
                        .or_insert_with(Vec::new)
                        .push(result.value);
                }
                map
            }
            Err(error) => {
                warn!(error = %error, "completion.smart_extractor_failed");
                HashMap::new()
            }
        };

        self.record_command_output_with_entities(
            command,
            output,
            working_directory,
            extracted_entities,
            timestamp,
        )
    }

    /// 记录命令输出（调用方已提供提取的实体）
    pub fn record_command_output_with_entities(
        &self,
        command: String,
        output: String,
        working_directory: String,
        extracted_entities: HashMap<String, Vec<String>>,
        timestamp: u64,
    ) -> CompletionProviderResult<()> {
        let record = CommandOutputRecord {
            command: command.clone(),
            output,
            timestamp,
            working_directory,
            extracted_entities,
        };

        let mut history =
            self.command_history
                .write()
                .map_err(|_| CompletionProviderError::MutexPoisoned {
                    resource: "command_history",
                })?;

        history.push(record);

        // 限制历史记录数量
        if history.len() > self.max_history {
            history.remove(0);
        }

        Ok(())
    }

    /// 获取相关的补全建议
    fn get_contextual_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 分析当前输入，确定需要什么类型的补全
        let current_command = context.current_word.clone();

        let history =
            self.command_history
                .read()
                .map_err(|_| CompletionProviderError::MutexPoisoned {
                    resource: "command_history",
                })?;

        // 根据当前命令类型提供相应的补全
        match &*current_command {
            "kill" | "killall" => {
                // 为 kill 命令提供 PID 补全
                items.extend(self.get_pid_completions(&history)?);
            }
            "nc" | "telnet" | "ssh" => {
                // 为网络命令提供端口和IP补全
                items.extend(self.get_network_completions(&history)?);
            }
            "cd" | "ls" | "cat" | "vim" | "nano" => {
                // 为文件操作命令提供路径补全（这里可以结合文件系统提供者）
                items.extend(self.get_path_completions(&history)?);
            }
            _ => {
                // 通用补全：查找相关的实体
                items.extend(self.get_general_completions(&history, &current_command)?);
            }
        }

        Ok(items)
    }

    /// 获取 PID 补全建议
    fn get_pid_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 查找最近的进程相关命令输出
        for record in history.iter().rev().take(10) {
            if let Some(pids) = record.extracted_entities.get("pid") {
                for pid in pids {
                    let description = if let Some(process_names) =
                        record.extracted_entities.get("process_name")
                    {
                        process_names.first().map(|name| format!("进程: {}", name))
                    } else {
                        Some("进程ID".to_string())
                    };

                    let item = CompletionItem::new(pid.clone(), CompletionType::Value)
                        .with_score(80.0) // 高分数，因为是上下文相关的
                        .with_description(description.unwrap_or_default())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "pid".to_string())
                        .with_metadata("from_command".to_string(), record.command.clone());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// 获取网络相关补全建议
    fn get_network_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for record in history.iter().rev().take(10) {
            // 添加端口补全
            if let Some(ports) = record.extracted_entities.get("port") {
                for port in ports {
                    let item = CompletionItem::new(port.clone(), CompletionType::Value)
                        .with_score(75.0)
                        .with_description("端口号".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "port".to_string());

                    items.push(item);
                }
            }

            // 添加IP地址补全
            if let Some(ips) = record.extracted_entities.get("ip") {
                for ip in ips {
                    let item = CompletionItem::new(ip.clone(), CompletionType::Value)
                        .with_score(75.0)
                        .with_description("IP地址".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "ip".to_string());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// 获取路径相关补全建议
    fn get_path_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for record in history.iter().rev().take(5) {
            if let Some(paths) = record.extracted_entities.get("path") {
                for path in paths {
                    let item = CompletionItem::new(path.clone(), CompletionType::File)
                        .with_score(70.0)
                        .with_description("文件路径".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "path".to_string());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// 获取通用补全建议
    fn get_general_completions(
        &self,
        history: &[CommandOutputRecord],
        command: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 基于命令类型提供基础补全
        match command {
            "cd" => {
                // 为cd命令提供目录补全
                if let Some(record) = history.last() {
                    if let Some(dirs) = record.extracted_entities.get("directory") {
                        for dir in dirs {
                            let item = CompletionItem::new(dir.clone(), CompletionType::Directory)
                                .with_score(60.0)
                                .with_description("目录".to_string())
                                .with_source("context".to_string());
                            items.push(item);
                        }
                    }
                }
            }
            "cat" | "less" | "more" | "head" | "tail" => {
                // 为文件查看命令提供文件补全
                if let Some(record) = history.last() {
                    if let Some(files) = record.extracted_entities.get("file") {
                        for file in files {
                            let item = CompletionItem::new(file.clone(), CompletionType::File)
                                .with_score(60.0)
                                .with_description("文件".to_string())
                                .with_source("context".to_string());
                            items.push(item);
                        }
                    }
                }
            }
            _ => {
                // 其他命令暂不提供特殊补全
            }
        }

        Ok(items)
    }
}

#[async_trait]
impl CompletionProvider for ContextAwareProvider {
    fn name(&self) -> &'static str {
        "context_aware"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // 总是尝试提供上下文感知的补全
        !context.current_word.is_empty()
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        self.get_contextual_completions(context)
    }

    fn priority(&self) -> i32 {
        20 // 最高优先级，因为是上下文相关的
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for ContextAwareProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// 上下文感知提供者包装器
/// 用于将 Arc<Mutex<ContextAwareProvider>> 适配为 Arc<dyn CompletionProvider>
pub struct ContextAwareProviderWrapper {
    provider: Arc<Mutex<ContextAwareProvider>>,
}

impl ContextAwareProviderWrapper {
    pub fn new(provider: Arc<Mutex<ContextAwareProvider>>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl CompletionProvider for ContextAwareProviderWrapper {
    fn name(&self) -> &'static str {
        "context_aware_wrapper"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        if let Ok(provider) = self.provider.lock() {
            provider.should_provide(context)
        } else {
            false
        }
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        // 克隆上下文以避免跨 await 边界的借用问题
        let context_clone = context.clone();

        let result = {
            if let Ok(provider) = self.provider.lock() {
                // 直接调用同步方法，避免异步调用
                provider.get_contextual_completions(&context_clone)
            } else {
                Ok(Vec::new())
            }
        };

        result
    }

    fn priority(&self) -> i32 {
        20 // 最高优先级
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
