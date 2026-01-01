/*!
 * 输出分析器模块
 *
 * 负责分析终端输出，提取有用的上下文信息用于智能补全
 */

use crate::completion::error::{OutputAnalyzerError, OutputAnalyzerResult};
use crate::completion::providers::ContextAwareProvider;
use crate::completion::smart_extractor::SmartExtractor;
use crate::mux::ConfigManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tracing::warn;

/// 全局输出分析器实例
static GLOBAL_OUTPUT_ANALYZER: OnceLock<Arc<OutputAnalyzer>> = OnceLock::new();

/// 面板缓冲区条目
#[derive(Debug, Clone)]
struct PaneBufferEntry {
    content: String,
    last_access: Instant,
    created_at: Instant,
}

impl PaneBufferEntry {
    fn new() -> Self {
        Self {
            content: String::new(),
            last_access: Instant::now(),
            created_at: Instant::now(),
        }
    }

    fn access(&mut self) {
        self.last_access = Instant::now();
    }

    fn is_stale(&self, max_age: Duration) -> bool {
        self.last_access.elapsed() > max_age
    }

    fn is_too_new(&self) -> bool {
        self.created_at.elapsed() < Duration::from_secs(2)
    }
}

/// 输出分析器
pub struct OutputAnalyzer {
    /// 上下文感知提供者
    context_provider: Arc<Mutex<ContextAwareProvider>>,
    /// 命令输出缓冲区 - 按面板ID存储，包含访问时间信息
    output_buffer: Arc<Mutex<HashMap<u32, PaneBufferEntry>>>,
    /// 最后清理时间
    last_global_cleanup: Arc<Mutex<Instant>>,
}

impl OutputAnalyzer {
    /// 创建新的输出分析器
    pub fn new() -> Self {
        Self {
            context_provider: Arc::new(Mutex::new(ContextAwareProvider::new())),
            output_buffer: Arc::new(Mutex::new(HashMap::new())),
            last_global_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 获取全局实例
    pub fn global() -> &'static Arc<OutputAnalyzer> {
        GLOBAL_OUTPUT_ANALYZER.get_or_init(|| Arc::new(OutputAnalyzer::new()))
    }

    /// 安全获取缓冲区锁，处理中毒情况
    fn get_buffer_lock(
        &self,
    ) -> OutputAnalyzerResult<MutexGuard<'_, HashMap<u32, PaneBufferEntry>>> {
        match self.output_buffer.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                warn!("输出缓冲区锁被中毒，尝试恢复数据");
                Ok(poisoned.into_inner())
            }
        }
    }

    /// 安全获取上下文提供者锁，处理中毒情况
    fn get_context_provider_lock(
        &self,
    ) -> OutputAnalyzerResult<MutexGuard<'_, ContextAwareProvider>> {
        match self.context_provider.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                warn!("上下文提供者锁被中毒，尝试恢复数据");
                Ok(poisoned.into_inner())
            }
        }
    }

    /// 分析终端输出
    pub fn analyze_output(&self, pane_id: u32, data: &str) -> OutputAnalyzerResult<()> {
        self.maybe_cleanup_stale_buffers()?;

        // 一次性处理所有缓冲区操作，避免多次获取锁
        let (should_process_commands, processed_output) = {
            let mut buffer = self.get_buffer_lock()?;

            let entry = buffer.entry(pane_id).or_insert_with(PaneBufferEntry::new);
            entry.access();

            // 添加新数据
            entry.content.push_str(data);

            // 安全截断缓冲区，防止内存泄漏和无限循环
            let config = ConfigManager::config_get();
            if entry.content.len() > config.buffer.max_size {
                self.safe_truncate_buffer(&mut entry.content)?;
            }

            let should_process = self.has_complete_command(&entry.content);
            let content_copy = if should_process {
                Some(entry.content.clone())
            } else {
                None
            };

            (should_process, content_copy)
        };

        // 在锁外处理命令，避免死锁
        if should_process_commands {
            if let Some(output) = processed_output {
                self.process_complete_commands(pane_id, &output)?;
            }
        }

        Ok(())
    }

    /// 安全截断缓冲区，防止无限循环
    fn safe_truncate_buffer(&self, content: &mut String) -> OutputAnalyzerResult<()> {
        let config = ConfigManager::config_get();
        let target_start = content.len().saturating_sub(config.buffer.keep_size);
        let mut byte_start = target_start;
        let mut attempts = 0;

        // 防止无限循环的安全检查
        while !content.is_char_boundary(byte_start)
            && byte_start < content.len()
            && attempts < config.buffer.max_truncation_attempts
        {
            byte_start += 1;
            attempts += 1;
        }

        if attempts >= config.buffer.max_truncation_attempts {
            warn!("字符边界查找超过最大尝试次数，使用保守截断策略");
            byte_start = content.len().saturating_sub(config.buffer.keep_size / 2);

            // 确保不会在UTF-8字符中间截断
            while byte_start > 0 && !content.is_char_boundary(byte_start) {
                byte_start -= 1;
            }
        }

        if byte_start > 0 && byte_start < content.len() {
            let truncated = content.split_off(byte_start);
            *content = truncated;
        }

        Ok(())
    }

    /// 检查是否有完整的命令
    fn has_complete_command(&self, content: &str) -> bool {
        // 简单检查：是否包含提示符模式
        content.lines().any(|line| {
            line.contains('$') || line.contains('#') || line.contains('%') || line.contains('>')
        })
    }

    /// 处理完整的命令（在锁外调用，避免死锁）
    fn process_complete_commands(&self, pane_id: u32, output: &str) -> OutputAnalyzerResult<()> {
        // 尝试检测命令
        if let Some((command, command_output)) = self.detect_command_completion(output) {
            self.process_complete_command(&command, &command_output)?;

            // 清理缓冲区中已处理的部分（一次性操作）
            {
                let mut buffer = self.get_buffer_lock()?;
                if let Some(entry) = buffer.get_mut(&pane_id) {
                    entry.access();

                    // 保留最后的提示符部分
                    if let Some(last_prompt_pos) = entry.content.rfind('$') {
                        entry.content = entry.content[last_prompt_pos..].to_string();
                    } else {
                        entry.content.clear();
                    }
                }
            }
        }

        Ok(())
    }

    /// 定期清理过期的缓冲区，防止内存泄漏
    fn maybe_cleanup_stale_buffers(&self) -> OutputAnalyzerResult<()> {
        let config = ConfigManager::config_get();
        let should_cleanup = {
            let cleanup_guard = self.last_global_cleanup.lock().map_err(|_| {
                OutputAnalyzerError::MutexPoisoned {
                    resource: "last_global_cleanup",
                }
            })?;
            cleanup_guard.elapsed() > config.cleanup_interval()
        };

        if should_cleanup && config.cleanup.auto_cleanup_enabled {
            self.cleanup_stale_buffers()?;
        }

        Ok(())
    }

    /// 清理过期的缓冲区
    fn cleanup_stale_buffers(&self) -> OutputAnalyzerResult<()> {
        let config = ConfigManager::config_get();
        let stale_threshold = config.stale_threshold();

        {
            let mut buffer = self.get_buffer_lock()?;
            let mut to_remove = Vec::new();

            // 收集需要删除的面板ID
            for (&pane_id, entry) in buffer.iter() {
                if entry.is_stale(stale_threshold) {
                    to_remove.push(pane_id);
                }
            }

            // 删除过期条目
            for pane_id in to_remove {
                buffer.remove(&pane_id);
            }
        }

        // 更新清理时间
        {
            let mut cleanup_guard = self.last_global_cleanup.lock().map_err(|_| {
                OutputAnalyzerError::MutexPoisoned {
                    resource: "last_global_cleanup",
                }
            })?;
            *cleanup_guard = Instant::now();
        }

        Ok(())
    }

    /// 清理指定面板的缓冲区（外部调用）
    pub fn cleanup_pane_buffer(&self, pane_id: u32) -> OutputAnalyzerResult<()> {
        let mut buffer = self.get_buffer_lock()?;
        buffer.remove(&pane_id);
        Ok(())
    }

    /// 检测命令完成
    fn detect_command_completion(&self, output: &str) -> Option<(String, String)> {
        // 改进的命令检测：查找命令行模式
        let lines: Vec<&str> = output.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i];

            // 检测命令行（包含 $ 或 # 提示符）
            if let Some(command_start) = self.find_command_in_line(line) {
                let command = line[command_start..].trim().to_string();

                // 收集命令输出（从下一行开始到下一个提示符或结尾）
                let mut command_output = String::new();
                for output_line in lines.iter().skip(i + 1) {
                    if self.is_prompt_line(output_line) {
                        break;
                    }

                    command_output.push_str(output_line);
                    command_output.push('\n');
                }

                if !command.is_empty() && self.is_command_complete(&command_output) {
                    return Some((command, command_output.trim().to_string()));
                }
            }
        }

        None
    }

    /// 在行中查找命令
    fn find_command_in_line(&self, line: &str) -> Option<usize> {
        // 查找提示符后的命令
        if let Some(pos) = line.find('$') {
            return Some(pos + 1);
        }
        if let Some(pos) = line.find('#') {
            return Some(pos + 1);
        }
        if let Some(pos) = line.find('%') {
            return Some(pos + 1);
        }
        if let Some(pos) = line.find('>') {
            return Some(pos + 1);
        }

        None
    }

    /// 检查是否是提示符行
    fn is_prompt_line(&self, line: &str) -> bool {
        line.contains('$') || line.contains('#') || line.contains('%') || line.contains('>')
    }

    /// 检查命令是否完整
    fn is_command_complete(&self, output: &str) -> bool {
        // 简单检测：如果输出不为空且不包含错误信息，认为完整
        !output.trim().is_empty()
            && !output.contains("command not found")
            && !output.contains("No such file or directory")
    }

    /// 处理完整的命令
    fn process_complete_command(&self, command: &str, output: &str) -> OutputAnalyzerResult<()> {
        // 使用智能提取器分析输出
        let extractor = SmartExtractor::global();
        let extraction_results = extractor.extract_entities(command, output)?;

        // 转换为标准格式
        let mut entities = HashMap::new();
        for result in extraction_results {
            entities
                .entry(result.entity_type)
                .or_insert_with(Vec::new)
                .push(result.value);
        }

        // 记录命令输出到上下文感知提供者
        let provider = self.get_context_provider_lock()?;

        let mut enhanced_output = output.to_string();
        if !entities.is_empty() {
            enhanced_output.push_str("\n\n<!-- 提取的实体: ");
            enhanced_output.push_str(&serde_json::to_string(&entities).unwrap_or_default());
            enhanced_output.push_str(" -->");
        }

        provider.record_command_output(
            command.to_string(),
            enhanced_output,
            "/tmp".to_string(), // 这里应该获取实际的工作目录
        )?;

        Ok(())
    }

    /// 获取上下文感知提供者的引用
    pub fn get_context_provider(&self) -> Arc<Mutex<ContextAwareProvider>> {
        Arc::clone(&self.context_provider)
    }

    /// 获取指定面板的缓冲区内容
    pub fn get_pane_buffer(&self, pane_id: u32) -> OutputAnalyzerResult<String> {
        let mut buffer = self.get_buffer_lock()?;

        if let Some(entry) = buffer.get_mut(&pane_id) {
            entry.access();
            Ok(entry.content.clone())
        } else {
            Ok(String::new())
        }
    }

    /// 检查指定面板的缓冲区是否太新（刚创建 <2秒）
    pub fn is_pane_buffer_too_new(&self, pane_id: u32) -> bool {
        if let Ok(buffer) = self.get_buffer_lock() {
            if let Some(entry) = buffer.get(&pane_id) {
                return entry.is_too_new();
            }
        }
        false
    }

    /// 设置指定面板的缓冲区内容
    pub fn set_pane_buffer(&self, pane_id: u32, content: String) -> OutputAnalyzerResult<()> {
        let mut buffer = self.get_buffer_lock()?;

        let entry = buffer.entry(pane_id).or_insert_with(PaneBufferEntry::new);
        entry.content = content;
        entry.access();

        Ok(())
    }

    /// 清理指定面板的缓冲区
    pub fn clear_pane_buffer(&self, pane_id: u32) -> OutputAnalyzerResult<()> {
        let mut buffer = self.get_buffer_lock()?;
        buffer.remove(&pane_id);
        Ok(())
    }

    /// 获取缓冲区统计信息
    pub fn get_buffer_stats(&self) -> OutputAnalyzerResult<HashMap<String, usize>> {
        let buffer = self.get_buffer_lock()?;

        let mut stats = HashMap::new();
        stats.insert("total_panes".to_string(), buffer.len());

        let total_size: usize = buffer.values().map(|entry| entry.content.len()).sum();
        stats.insert("total_buffer_size".to_string(), total_size);

        let avg_size = if buffer.is_empty() {
            0
        } else {
            total_size / buffer.len()
        };
        stats.insert("average_buffer_size".to_string(), avg_size);

        Ok(stats)
    }
}

impl Default for OutputAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
