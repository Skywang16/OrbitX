use serde::{Deserialize, Serialize};

/// 消息中的图片附件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMessageImage {
    pub id: String,
    pub data_url: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiStep {
    pub step_type: String,
    pub content: String,
    pub timestamp: i64,
    pub metadata: Option<serde_json::Value>,
}
