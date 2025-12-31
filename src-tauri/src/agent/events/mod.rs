/*!
 * TaskExecutor事件类型定义
 *
 * 定义任务执行过程中的各种进度事件，用于实时状态同步
 */

use crate::agent::core::status::AgentTaskStatus;
use crate::llm::types::LLMUsage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ===== 每个事件的载荷结构（统一 camelCase 字段） =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCreatedPayload {
    pub task_id: String,
    pub session_id: i64,
    pub workspace_path: String,
    pub user_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPayload {
    pub task_id: String,
    pub iteration: u32,
    pub text: String,
    pub stream_id: String,
    pub stream_done: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusChangedPayload {
    pub task_id: String,
    pub status: AgentTaskStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStartedPayload {
    pub task_id: String,
    pub iteration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingPayload {
    pub task_id: String,
    pub iteration: u32,
    pub thought: String,
    pub stream_id: String,
    pub stream_done: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsePayload {
    pub task_id: String,
    pub iteration: u32,
    pub tool_id: String,
    pub tool_name: String,
    pub params: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultPayload {
    pub task_id: String,
    pub iteration: u32,
    pub tool_id: String,
    pub tool_name: String,
    pub result: serde_json::Value,
    pub is_error: bool,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalAnswerPayload {
    pub task_id: String,
    pub iteration: u32,
    pub answer: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishPayload {
    pub task_id: String,
    pub iteration: u32,
    pub finish_reason: String,
    pub usage: Option<LLMUsage>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPausedPayload {
    pub task_id: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResumedPayload {
    pub task_id: String,
    pub from_iteration: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompletedPayload {
    pub task_id: String,
    pub final_iteration: u32,
    pub completion_reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskErrorPayload {
    pub task_id: String,
    pub iteration: u32,
    pub error_message: String,
    pub error_type: String,
    pub is_recoverable: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCancelledPayload {
    pub task_id: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdatePayload {
    pub task_id: String,
    pub status: String,
    pub current_iteration: u32,
    pub error_count: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessagePayload {
    pub task_id: String,
    pub message: String,
    pub level: String, // "info", "warning", "error"
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPreparingPayload {
    pub tool_name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub message: String,
    pub recoverable: bool,
}

/// 任务进度事件负载
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "PascalCase")]
pub enum TaskProgressPayload {
    /// 任务已创建
    TaskCreated(TaskCreatedPayload),

    /// 状态变更
    StatusChanged(StatusChangedPayload),

    /// 任务开始执行
    TaskStarted(TaskStartedPayload),

    /// Agent正在思考
    Thinking(ThinkingPayload),

    /// 开始调用工具
    ToolUse(ToolUsePayload),

    /// 工具调用结果
    ToolResult(ToolResultPayload),

    /// 最终答案
    FinalAnswer(FinalAnswerPayload),

    /// 文本流
    Text(TextPayload),

    /// 结束事件（包含usage）
    Finish(FinishPayload),

    /// 任务暂停
    TaskPaused(TaskPausedPayload),

    /// 任务恢复
    TaskResumed(TaskResumedPayload),

    /// 任务完成
    TaskCompleted(TaskCompletedPayload),

    /// 任务错误
    TaskError(TaskErrorPayload),

    /// 任务取消
    TaskCancelled(TaskCancelledPayload),

    /// 状态更新
    StatusUpdate(StatusUpdatePayload),

    /// 系统消息
    SystemMessage(SystemMessagePayload),

    /// 准备调用工具（无 task_id）
    ToolPreparing(ToolPreparingPayload),

    /// 通用错误事件（无 task_id）
    Error(ErrorPayload),
}

impl TaskProgressPayload {
    /// 获取事件的任务ID
    pub fn task_id(&self) -> &str {
        match self {
            TaskProgressPayload::TaskCreated(p) => &p.task_id,
            TaskProgressPayload::StatusChanged(p) => &p.task_id,
            TaskProgressPayload::TaskStarted(p) => &p.task_id,
            TaskProgressPayload::Thinking(p) => &p.task_id,
            TaskProgressPayload::ToolUse(p) => &p.task_id,
            TaskProgressPayload::ToolResult(p) => &p.task_id,
            TaskProgressPayload::FinalAnswer(p) => &p.task_id,
            TaskProgressPayload::Finish(p) => &p.task_id,
            TaskProgressPayload::TaskPaused(p) => &p.task_id,
            TaskProgressPayload::TaskResumed(p) => &p.task_id,
            TaskProgressPayload::TaskCompleted(p) => &p.task_id,
            TaskProgressPayload::TaskError(p) => &p.task_id,
            TaskProgressPayload::TaskCancelled(p) => &p.task_id,
            TaskProgressPayload::StatusUpdate(p) => &p.task_id,
            TaskProgressPayload::SystemMessage(p) => &p.task_id,
            TaskProgressPayload::Text(p) => &p.task_id,
            TaskProgressPayload::ToolPreparing(_) => "", // No task_id for this event
            TaskProgressPayload::Error(_) => "",         // No task_id for this event
        }
    }

    /// 获取事件的时间戳
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            TaskProgressPayload::TaskCreated(_) => Utc::now(),
            TaskProgressPayload::StatusChanged(p) => p.timestamp,
            TaskProgressPayload::TaskStarted(_) => Utc::now(),
            TaskProgressPayload::Thinking(p) => p.timestamp,
            TaskProgressPayload::ToolUse(p) => p.timestamp,
            TaskProgressPayload::ToolResult(p) => p.timestamp,
            TaskProgressPayload::FinalAnswer(p) => p.timestamp,
            TaskProgressPayload::Finish(p) => p.timestamp,
            TaskProgressPayload::TaskPaused(p) => p.timestamp,
            TaskProgressPayload::TaskResumed(p) => p.timestamp,
            TaskProgressPayload::TaskCompleted(p) => p.timestamp,
            TaskProgressPayload::TaskError(p) => p.timestamp,
            TaskProgressPayload::TaskCancelled(p) => p.timestamp,
            TaskProgressPayload::StatusUpdate(p) => p.timestamp,
            TaskProgressPayload::SystemMessage(p) => p.timestamp,
            TaskProgressPayload::Text(p) => p.timestamp,
            TaskProgressPayload::ToolPreparing(_) => Utc::now(),
            TaskProgressPayload::Error(_) => Utc::now(),
        }
    }

    /// 判断是否为错误事件
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            TaskProgressPayload::TaskError(_) | TaskProgressPayload::Error(_)
        )
    }

    /// 判断是否为终止事件
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskProgressPayload::TaskCompleted(_)
                | TaskProgressPayload::TaskCancelled(_)
                | TaskProgressPayload::TaskError(TaskErrorPayload {
                    is_recoverable: false,
                    ..
                })
        )
    }
}
