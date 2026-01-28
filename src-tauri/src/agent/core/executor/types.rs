/*!
 * TaskExecutor类型定义
 */

use serde::{Deserialize, Serialize};

/// 图片附件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachment {
    #[serde(rename = "type")]
    pub attachment_type: String,
    pub data_url: String,
    pub mime_type: String,
}

/// 任务执行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteTaskParams {
    /// 归属工作区（绝对路径/规范化）
    pub workspace_path: String,
    /// 会话 ID（工作区下的 session）
    pub session_id: i64,
    pub user_prompt: String,
    pub model_id: String,
    /// Optional per-request agent type override (does not persist to session).
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<ImageAttachment>>,
    /// Runtime system reminders to inject into LLM context (not persisted to UI messages).
    /// These are wrapped in <system-reminder> tags when sent to the LLM.
    #[serde(skip, default)]
    pub system_reminders: Vec<String>,
}

/// 任务摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub task_id: String,
    pub session_id: i64,
    pub status: String,
    pub current_iteration: i32,
    pub error_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 文件上下文状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContextStatus {
    pub workspace_path: String,
    pub file_count: usize,
    pub files: Vec<String>,
}
