/*!
 * 输出分析器模块
 *
 * 负责分析终端输出，提取有用的上下文信息用于智能补全和 replay
 */

use crate::completion::command_line::normalize_command_line;
use crate::completion::error::OutputAnalyzerResult;
use crate::completion::providers::ContextAwareProvider;
use crate::completion::smart_extractor::SmartExtractor;
use crate::completion::CompletionRuntime;
use crate::mux::ConfigManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

struct HistoryBufferEntry {
    content: String,
    created_at: Instant,
}

impl HistoryBufferEntry {
    fn new() -> Self {
        Self {
            content: String::new(),
            created_at: Instant::now(),
        }
    }

    fn is_too_new(&self) -> bool {
        self.created_at.elapsed() < std::time::Duration::from_secs(2)
    }

    fn append(&mut self, data: &str, max_size: usize) {
        self.content.push_str(data);

        if self.content.len() > max_size {
            let keep_size = max_size / 2;
            let start = self.content.len().saturating_sub(keep_size);

            // 高效地找到 UTF-8 字符边界：从 start 位置向后扫描
            // UTF-8 字符的首字节不会是 10xxxxxx (0x80-0xBF)
            let byte_start = self.content.as_bytes()[start..]
                .iter()
                .position(|&b| (b & 0xC0) != 0x80)
                .map(|offset| start + offset)
                .unwrap_or(self.content.len());

            // 使用 drain 避免额外分配
            self.content.drain(..byte_start);
        }
    }
}

struct OutputAnalyzerInner {
    history_buffer: HashMap<u32, HistoryBufferEntry>,
    active_command_ids: HashMap<u32, u64>,
}

pub struct OutputAnalyzer {
    context_provider: Arc<ContextAwareProvider>,
    inner: Mutex<OutputAnalyzerInner>,
}

impl OutputAnalyzer {
    pub fn new() -> Self {
        Self {
            context_provider: Arc::new(ContextAwareProvider::new()),
            inner: Mutex::new(OutputAnalyzerInner {
                history_buffer: HashMap::new(),
                active_command_ids: HashMap::new(),
            }),
        }
    }

    pub fn global() -> &'static Arc<OutputAnalyzer> {
        CompletionRuntime::global().output_analyzer()
    }

    fn lock_inner(&self) -> OutputAnalyzerResult<MutexGuard<'_, OutputAnalyzerInner>> {
        match self.inner.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => Ok(poisoned.into_inner()),
        }
    }

    /// 分析终端输出，写入历史缓冲区
    pub fn analyze_output(&self, pane_id: u32, data: &str) -> OutputAnalyzerResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        let config = ConfigManager::config_get();

        let should_process = {
            let mut inner = self.lock_inner()?;
            let entry = inner
                .history_buffer
                .entry(pane_id)
                .or_insert_with(HistoryBufferEntry::new);

            // 直接检查新数据中是否有命令完成标志
            // 避免在 append 后使用可能失效的索引
            let has_prompt = self.has_complete_command(data);
            entry.append(data, config.buffer.max_size);
            has_prompt
        };

        if should_process {
            if let Some((command, output)) = self
                .get_pane_buffer(pane_id)
                .ok()
                .and_then(|content| self.detect_command_completion(&content))
            {
                // 无 Shell Integration 的 fallback 路径：从 prompt/输出中猜测命令边界
                self.record_completed_command(pane_id, command, output, "/tmp".to_string())?;
            }
        }

        Ok(())
    }

    /// 处理来自 Shell Integration 的命令事件：可靠的“上一条命令”来源
    ///
    /// 设计要点：
    /// - 命令开始时清空缓冲区，确保后续缓冲内容只属于该命令
    /// - 命令结束时将缓冲区作为输出记录，用于预测/实体补全
    pub fn on_shell_command_event(
        &self,
        pane_id: u32,
        command: &crate::shell::CommandInfo,
    ) -> OutputAnalyzerResult<()> {
        let Some(command_line) = command
            .command_line
            .as_deref()
            .and_then(normalize_command_line)
        else {
            return Ok(());
        };

        if command.is_finished() {
            let output = self.get_pane_buffer(pane_id).unwrap_or_default();
            let cwd = command
                .working_directory
                .as_deref()
                .unwrap_or("/tmp")
                .to_string();

            self.record_completed_command(pane_id, command_line.to_string(), output, cwd)?;
            self.clear_pane_buffer(pane_id)?;

            let mut inner = self.lock_inner()?;
            inner.active_command_ids.remove(&pane_id);
            return Ok(());
        }

        // Running：只在 command id 切换时清空一次，避免重复清空导致丢输出
        let should_clear = {
            let mut inner = self.lock_inner()?;
            match inner.active_command_ids.get(&pane_id).copied() {
                Some(id) if id == command.id => false,
                _ => {
                    inner.active_command_ids.insert(pane_id, command.id);
                    true
                }
            }
        };

        if should_clear {
            self.clear_pane_buffer(pane_id)?;
        }

        Ok(())
    }

    fn has_complete_command(&self, content: &str) -> bool {
        content.lines().any(|line| {
            line.contains('$') || line.contains('#') || line.contains('%') || line.contains('>')
        })
    }

    pub fn get_pane_buffer(&self, pane_id: u32) -> OutputAnalyzerResult<String> {
        let inner = self.lock_inner()?;

        if let Some(entry) = inner.history_buffer.get(&pane_id) {
            Ok(entry.content.clone())
        } else {
            Ok(String::new())
        }
    }

    pub fn is_pane_buffer_too_new(&self, pane_id: u32) -> bool {
        if let Ok(inner) = self.lock_inner() {
            if let Some(entry) = inner.history_buffer.get(&pane_id) {
                return entry.is_too_new();
            }
        }
        false
    }

    pub fn clear_pane_buffer(&self, pane_id: u32) -> OutputAnalyzerResult<()> {
        let mut inner = self.lock_inner()?;
        inner.history_buffer.remove(&pane_id);
        Ok(())
    }

    pub fn get_buffer_stats(&self) -> OutputAnalyzerResult<HashMap<String, usize>> {
        let inner = self.lock_inner()?;

        let mut stats = HashMap::new();
        stats.insert("total_panes".to_string(), inner.history_buffer.len());
        stats.insert(
            "history_buffer_size".to_string(),
            inner.history_buffer.values().map(|e| e.content.len()).sum(),
        );

        Ok(stats)
    }

    fn detect_command_completion(&self, output: &str) -> Option<(String, String)> {
        let lines: Vec<&str> = output.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i];

            if let Some(command_start) = self.find_command_in_line(line) {
                // 安全切片：find_command_in_line 返回的是字节索引
                let command = line
                    .get(command_start..)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

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

    fn find_command_in_line(&self, line: &str) -> Option<usize> {
        line.find('$')
            .or_else(|| line.find('#'))
            .or_else(|| line.find('%'))
            .or_else(|| line.find('>'))
            .map(|p| p + 1)
    }

    fn is_prompt_line(&self, line: &str) -> bool {
        line.contains('$') || line.contains('#') || line.contains('%') || line.contains('>')
    }

    fn is_command_complete(&self, output: &str) -> bool {
        !output.trim().is_empty()
            && !output.contains("command not found")
            && !output.contains("No such file or directory")
    }

    fn record_completed_command(
        &self,
        _pane_id: u32,
        command: String,
        output: String,
        working_directory: String,
    ) -> OutputAnalyzerResult<()> {
        let extractor = SmartExtractor::global();
        let extraction_results = extractor.extract_entities(&command, &output)?;

        let mut entities = HashMap::new();
        for result in extraction_results {
            entities
                .entry(result.entity_type)
                .or_insert_with(Vec::new)
                .push(result.value);
        }

        self.context_provider.record_command_output_with_entities(
            command,
            output,
            working_directory,
            entities,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        )?;

        Ok(())
    }

    pub fn context_provider(&self) -> Arc<ContextAwareProvider> {
        Arc::clone(&self.context_provider)
    }
}

impl Default for OutputAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::osc_parser::CommandStatus;
    use crate::shell::CommandInfo;

    #[test]
    fn test_shell_command_event_records_last_command() {
        let analyzer = OutputAnalyzer::new();
        let pane_id = 1u32;

        let mut cmd = CommandInfo {
            id: 42,
            start_time: Instant::now(),
            start_time_wallclock: std::time::SystemTime::now(),
            end_time: None,
            end_time_wallclock: None,
            exit_code: None,
            status: CommandStatus::Running,
            command_line: Some("wangjiajie@host % git status".to_string()),
            working_directory: Some("/tmp".to_string()),
        };

        analyzer.on_shell_command_event(pane_id, &cmd).unwrap();
        analyzer
            .analyze_output(pane_id, "On branch main\n")
            .unwrap();

        cmd.status = CommandStatus::Finished { exit_code: Some(0) };
        analyzer.on_shell_command_event(pane_id, &cmd).unwrap();

        let provider = analyzer.context_provider();
        let (last_cmd, last_output) = provider.get_last_command().unwrap();

        assert_eq!(last_cmd, "git status");
        assert!(last_output.contains("On branch main"));
    }
}
