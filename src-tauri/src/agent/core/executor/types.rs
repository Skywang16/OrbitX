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
    pub conversation_id: i64,
    pub user_prompt: String,
    pub chat_mode: String,
    pub model_id: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<ImageAttachment>>,
}

/// 任务摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub task_id: String,
    pub conversation_id: i64,
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
    pub conversation_id: i64,
    pub file_count: usize,
    pub files: Vec<String>,
}
