use serde::{Deserialize, Serialize};

/// 空数据类型，用于无数据返回的API
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EmptyData;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// 成功响应（无消息）
    pub fn ok(data: T) -> Self {
        Self {
            code: 200,
            message: None,
            data: Some(data),
        }
    }

    /// 成功响应（带消息）
    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: 200,
            message: Some(message.into()),
            data: Some(data),
        }
    }

    /// 错误响应（已完成 i18n）
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: 500,
            message: Some(message.into()),
            data: None,
        }
    }
}

/// Tauri 命令专用结果类型
pub type TauriApiResult<T> = Result<ApiResponse<T>, String>;
