/*!
 * 上下文管理器模块
 */

use crate::ai::{AIContext, SystemInfo};
use crate::storage::sqlite::CommandHistoryEntry;
use crate::utils::error::AppResult;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 上下文统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextStats {
    pub total_commands: usize,
    pub unique_commands: usize,
    pub average_command_length: f64,
    pub most_used_commands: Vec<(String, usize)>,
    pub recent_activity: Vec<CommandHistoryEntry>,
}

/// 上下文管理器
pub struct ContextManager {
    max_history_size: usize,
    command_history: VecDeque<CommandHistoryEntry>,
    current_context: AIContext,
    command_frequency: HashMap<String, usize>,
    session_start_time: u64,
}

// CommandHistoryEntry的实现方法现在在storage::sqlite模块中

impl ContextManager {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            max_history_size,
            command_history: VecDeque::new(),
            current_context: AIContext::default(),
            command_frequency: HashMap::new(),
            session_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// 更新工作目录
    pub fn update_working_directory(&mut self, directory: String) {
        self.current_context.working_directory = Some(directory);
    }

    /// 添加命令到历史
    pub fn add_command(&mut self, command: String) {
        if self.command_history.len() >= self.max_history_size {
            self.command_history.pop_front();
        }

        // 更新命令频率统计
        *self.command_frequency.entry(command.clone()).or_insert(0) += 1;

        // 创建适配的CommandHistoryEntry
        let entry = CommandHistoryEntry {
            id: None,
            command,
            working_directory: self
                .current_context
                .working_directory
                .clone()
                .unwrap_or_default(),
            exit_code: None,
            output: None,
            duration_ms: None,
            executed_at: chrono::Utc::now(),
            session_id: None,
            tags: None,
        };

        self.command_history.push_back(entry);
        self.update_command_history();
    }

    /// 添加带详细信息的命令
    pub fn add_command_with_details(&mut self, entry: CommandHistoryEntry) {
        if self.command_history.len() >= self.max_history_size {
            self.command_history.pop_front();
        }

        // 更新命令频率统计
        *self
            .command_frequency
            .entry(entry.command.clone())
            .or_insert(0) += 1;

        self.command_history.push_back(entry);
        self.update_command_history();
    }

    /// 设置当前命令
    pub fn set_current_command(&mut self, command: Option<String>) {
        self.current_context.current_command = command;
    }

    /// 设置最后输出
    pub fn set_last_output(&mut self, output: Option<String>) {
        self.current_context.last_output = output;
    }

    /// 更新环境变量
    pub fn update_environment(&mut self, env: std::collections::HashMap<String, String>) {
        self.current_context.environment = Some(env);
    }

    /// 更新系统信息
    pub fn update_system_info(&mut self, system_info: SystemInfo) {
        self.current_context.system_info = Some(system_info);
    }

    /// 获取当前上下文
    pub fn get_context(&self) -> &AIContext {
        &self.current_context
    }

    /// 获取压缩的上下文（用于减少token使用）
    pub fn get_compressed_context(&self, max_history_items: usize) -> AIContext {
        let mut context = self.current_context.clone();

        if let Some(history) = &context.command_history {
            if history.len() > max_history_items {
                let start_index = history.len() - max_history_items;
                context.command_history = Some(history[start_index..].to_vec());
            }
        }

        context
    }

    /// 获取相关的历史命令
    pub fn get_relevant_history(&self, query: &str, max_items: usize) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut relevant: Vec<(String, f32)> = self
            .command_history
            .iter()
            .filter_map(|entry| {
                let cmd_lower = entry.command.to_lowercase();
                if cmd_lower.contains(&query_lower) {
                    // 简单的相关性评分
                    let score = if cmd_lower.starts_with(&query_lower) {
                        1.0
                    } else if cmd_lower.contains(&query_lower) {
                        0.5
                    } else {
                        0.1
                    };
                    Some((entry.command.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // 按相关性排序
        relevant.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        relevant
            .into_iter()
            .take(max_items)
            .map(|(cmd, _)| cmd)
            .collect()
    }

    /// 清除历史
    pub fn clear_history(&mut self) {
        self.command_history.clear();
        self.update_command_history();
    }

    /// 更新上下文中的命令历史
    fn update_command_history(&mut self) {
        self.current_context.command_history = Some(
            self.command_history
                .iter()
                .map(|entry| entry.command.clone())
                .collect(),
        );
    }

    /// 获取上下文统计信息
    pub fn get_stats(&self) -> ContextStats {
        let total_commands = self.command_history.len();
        let unique_commands = self.command_frequency.len();

        let average_command_length = if total_commands > 0 {
            self.command_history
                .iter()
                .map(|entry| entry.command.len())
                .sum::<usize>() as f64
                / total_commands as f64
        } else {
            0.0
        };

        // 获取最常用的命令
        let mut most_used: Vec<(String, usize)> = self
            .command_frequency
            .iter()
            .map(|(cmd, count)| (cmd.clone(), *count))
            .collect();
        most_used.sort_by(|a, b| b.1.cmp(&a.1));
        most_used.truncate(10); // 只保留前10个

        // 获取最近的活动
        let recent_activity: Vec<CommandHistoryEntry> = self
            .command_history
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect();

        ContextStats {
            total_commands,
            unique_commands,
            average_command_length,
            most_used_commands: most_used,
            recent_activity,
        }
    }

    /// 获取命令使用频率
    pub fn get_command_frequency(&self, command: &str) -> usize {
        self.command_frequency.get(command).copied().unwrap_or(0)
    }

    /// 获取最常用的命令
    pub fn get_most_used_commands(&self, limit: usize) -> Vec<(String, usize)> {
        let mut commands: Vec<(String, usize)> = self
            .command_frequency
            .iter()
            .map(|(cmd, count)| (cmd.clone(), *count))
            .collect();
        commands.sort_by(|a, b| b.1.cmp(&a.1));
        commands.truncate(limit);
        commands
    }

    /// 获取会话持续时间
    pub fn get_session_duration(&self) -> Duration {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Duration::from_secs(current_time - self.session_start_time)
    }

    /// 搜索命令历史
    pub fn search_history(&self, pattern: &str, case_sensitive: bool) -> Vec<CommandHistoryEntry> {
        let search_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        self.command_history
            .iter()
            .filter(|entry| {
                let command = if case_sensitive {
                    entry.command.clone()
                } else {
                    entry.command.to_lowercase()
                };
                command.contains(&search_pattern)
            })
            .cloned()
            .collect()
    }

    /// 获取命令执行统计
    pub fn get_execution_stats(&self) -> (usize, usize, f64) {
        let total_commands = self.command_history.len();
        let successful_commands = self
            .command_history
            .iter()
            .filter(|entry| entry.exit_code == Some(0))
            .count();

        let success_rate = if total_commands > 0 {
            successful_commands as f64 / total_commands as f64 * 100.0
        } else {
            0.0
        };

        (total_commands, successful_commands, success_rate)
    }

    /// 清除旧的历史记录
    pub fn cleanup_old_history(&mut self, max_age_seconds: u64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff_time = current_time - max_age_seconds;

        // 移除旧的命令历史
        self.command_history
            .retain(|entry| entry.executed_at.timestamp() as u64 > cutoff_time);

        // 重新计算命令频率
        self.command_frequency.clear();
        for entry in &self.command_history {
            *self
                .command_frequency
                .entry(entry.command.clone())
                .or_insert(0) += 1;
        }

        self.update_command_history();
    }

    /// 导出历史记录
    pub fn export_history(&self) -> AppResult<String> {
        serde_json::to_string_pretty(&self.command_history.iter().collect::<Vec<_>>())
            .with_context(|| "AI存储错误: Failed to export history")
    }

    /// 导入历史记录
    pub fn import_history(&mut self, json_data: &str) -> AppResult<()> {
        let entries: Vec<CommandHistoryEntry> = serde_json::from_str(json_data)
            .with_context(|| "AI存储错误: Failed to parse history JSON")?;

        for entry in entries {
            self.add_command_with_details(entry);
        }

        Ok(())
    }
}

impl AIContext {}
