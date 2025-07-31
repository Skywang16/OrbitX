//! 输出分析器模块
//!
//! 负责分析终端输出，提取有用的上下文信息用于智能补全

use crate::completion::providers::ContextAwareProvider;
use crate::completion::smart_extractor::SmartExtractor;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 全局输出分析器实例
static GLOBAL_OUTPUT_ANALYZER: OnceLock<Arc<OutputAnalyzer>> = OnceLock::new();

/// 输出分析器
pub struct OutputAnalyzer {
    /// 上下文感知提供者
    context_provider: Arc<Mutex<ContextAwareProvider>>,
    /// 命令输出缓冲区 - 按面板ID存储
    output_buffer: Arc<Mutex<HashMap<u32, String>>>,
}

impl OutputAnalyzer {
    /// 创建新的输出分析器
    pub fn new() -> Self {
        Self {
            context_provider: Arc::new(Mutex::new(ContextAwareProvider::new())),
            output_buffer: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取全局实例
    pub fn global() -> &'static Arc<OutputAnalyzer> {
        GLOBAL_OUTPUT_ANALYZER.get_or_init(|| Arc::new(OutputAnalyzer::new()))
    }

    /// 分析终端输出
    pub fn analyze_output(&self, pane_id: u32, data: &str) -> AppResult<()> {
        // 将输出添加到缓冲区
        {
            let mut buffer = self
                .output_buffer
                .lock()
                .map_err(|_| anyhow!("获取输出缓冲区锁失败"))?;

            let current_output = buffer.entry(pane_id).or_insert_with(String::new);
            current_output.push_str(data);

            // 限制缓冲区大小，避免内存泄漏
            if current_output.len() > 50000 {
                let start = current_output.len() - 25000;
                *current_output = current_output[start..].to_string();
            }
        }

        // 检查是否有完整的命令输出
        self.check_and_process_complete_commands(pane_id)?;

        Ok(())
    }

    /// 检查并处理完整的命令
    fn check_and_process_complete_commands(&self, pane_id: u32) -> AppResult<()> {
        let output = {
            let buffer = self
                .output_buffer
                .lock()
                .map_err(|_| anyhow!("获取输出缓冲区锁失败"))?;

            buffer.get(&pane_id).cloned().unwrap_or_default()
        };

        // 尝试检测命令
        if let Some((command, command_output)) = self.detect_command_completion(&output) {
            // 处理完整的命令
            self.process_complete_command(&command, &command_output)?;

            // 清理缓冲区中已处理的部分
            let mut buffer = self
                .output_buffer
                .lock()
                .map_err(|_| anyhow!("获取输出缓冲区锁失败"))?;

            if let Some(current_output) = buffer.get_mut(&pane_id) {
                // 保留最后的提示符部分
                if let Some(last_prompt_pos) = current_output.rfind('$') {
                    *current_output = current_output[last_prompt_pos..].to_string();
                } else {
                    current_output.clear();
                }
            }
        }

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
                    // 如果遇到新的提示符，停止收集
                    if self.is_prompt_line(output_line) {
                        break;
                    }

                    command_output.push_str(output_line);
                    command_output.push('\n');
                }

                // 检查命令是否完整
                if !command.is_empty() && self.is_command_complete(&command, &command_output) {
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
    fn is_command_complete(&self, _command: &str, output: &str) -> bool {
        // 简单检测：如果输出不为空且不包含错误信息，认为完整
        !output.trim().is_empty()
            && !output.contains("command not found")
            && !output.contains("No such file or directory")
    }

    /// 处理完整的命令
    fn process_complete_command(&self, command: &str, output: &str) -> AppResult<()> {
        // 使用智能提取器分析输出
        let extractor = SmartExtractor::global();
        let extraction_results = extractor.extract_entities(command, output)?;

        // 转换为旧格式以兼容现有代码
        let mut entities = HashMap::new();
        for result in extraction_results {
            entities
                .entry(result.entity_type)
                .or_insert_with(Vec::new)
                .push(result.value);
        }

        // 记录命令输出到上下文感知提供者
        let provider = self
            .context_provider
            .lock()
            .map_err(|_| anyhow!("获取上下文提供者锁失败"))?;

        // 创建一个增强的命令输出记录，包含提取的实体
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

    /// 清理指定面板的缓冲区
    pub fn clear_pane_buffer(&self, pane_id: u32) -> AppResult<()> {
        let mut buffer = self
            .output_buffer
            .lock()
            .map_err(|_| anyhow!("获取输出缓冲区锁失败"))?;

        buffer.remove(&pane_id);
        Ok(())
    }
}

impl Default for OutputAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
