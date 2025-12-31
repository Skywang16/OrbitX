//! Shell 执行相关类型定义

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use super::OutputRingBuffer;

/// 命令 ID 类型
pub type CommandId = u64;

/// 命令执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommandStatus {
    /// 等待执行
    Pending,
    /// 正在运行
    Running {
        #[serde(skip_serializing_if = "Option::is_none")]
        pid: Option<u32>,
    },
    /// 已完成
    Completed { exit_code: i32, duration_ms: u64 },
    /// 超时终止
    TimedOut { duration_ms: u64 },
    /// 被中止
    Aborted,
    /// 执行失败
    Failed { error: String },
}

impl CommandStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CommandStatus::Completed { .. }
                | CommandStatus::TimedOut { .. }
                | CommandStatus::Aborted
                | CommandStatus::Failed { .. }
        )
    }
}

/// 运行中的命令信息
pub struct RunningCommand {
    /// 命令 ID
    pub id: CommandId,
    /// 原始命令字符串
    pub command: String,
    /// 工作目录
    pub cwd: String,
    /// 开始时间
    pub started_at: Instant,
    /// 执行状态
    pub status: CommandStatus,
    /// 输出缓冲区
    pub output_buffer: OutputRingBuffer,
    /// 是否后台运行
    pub is_background: bool,
    /// 中止信号
    pub abort_signal: Arc<AtomicBool>,
    /// 进程 ID
    pub pid: Option<u32>,
}

impl RunningCommand {
    pub fn new(
        id: CommandId,
        command: String,
        cwd: String,
        is_background: bool,
        buffer_capacity: usize,
    ) -> Self {
        Self {
            id,
            command,
            cwd,
            started_at: Instant::now(),
            status: CommandStatus::Pending,
            output_buffer: OutputRingBuffer::new(buffer_capacity),
            is_background,
            abort_signal: Arc::new(AtomicBool::new(false)),
            pid: None,
        }
    }

    /// 获取已运行时长（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}

/// 运行中命令的简要信息（用于查询）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningCommandInfo {
    pub id: CommandId,
    pub command: String,
    pub cwd: String,
    pub status: CommandStatus,
    pub is_background: bool,
    pub elapsed_ms: u64,
    pub pid: Option<u32>,
}

impl From<&RunningCommand> for RunningCommandInfo {
    fn from(cmd: &RunningCommand) -> Self {
        Self {
            id: cmd.id,
            command: cmd.command.clone(),
            cwd: cmd.cwd.clone(),
            status: cmd.status.clone(),
            is_background: cmd.is_background,
            elapsed_ms: cmd.elapsed_ms(),
            pid: cmd.pid,
        }
    }
}

/// Shell 执行结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecutionResult {
    /// 命令 ID
    pub command_id: CommandId,
    /// 执行状态
    pub status: CommandStatus,
    /// 命令输出
    pub output: String,
    /// 退出码
    pub exit_code: Option<i32>,
    /// 执行时长毫秒
    pub duration_ms: u64,
    /// 工作目录
    pub cwd: String,
    /// 输出是否被截断
    pub output_truncated: bool,
}
