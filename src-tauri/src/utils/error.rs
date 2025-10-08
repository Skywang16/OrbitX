use anyhow::{anyhow, Result as AnyhowResult};

pub type AppResult<T> = AnyhowResult<T>;

pub type AppError = anyhow::Error;

pub type TauriResult<T> = Result<T, String>;

pub fn app_error(msg: impl Into<String>) -> AppError {
    anyhow!(msg.into())
}

/// 创建带上下文的错误转换函数
pub fn app_error_with_context<T>(msg: &str) -> impl FnOnce(T) -> AppError + '_
where
    T: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    move |err| anyhow!("{}: {}", msg, err)
}

pub fn to_tauri_result<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

pub trait ToTauriResult<T> {
    fn to_tauri(self) -> Result<T, String>;
}

impl<T> ToTauriResult<T> for AppResult<T> {
    fn to_tauri(self) -> Result<T, String> {
        to_tauri_result(self)
    }
}

#[macro_export]
macro_rules! app_bail {
    ($msg:literal $(,)?) => {
        return Err(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        return Err(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(anyhow::anyhow!($fmt, $($arg)*))
    };
}

/// 快速创建错误
#[macro_export]
macro_rules! app_error_msg {
    ($msg:literal $(,)?) => {
        anyhow::anyhow!($msg)
    };
    ($err:expr $(,)?) => {
        anyhow::anyhow!($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        anyhow::anyhow!($fmt, $($arg)*)
    };
}

pub struct Validator;

impl Validator {
    pub fn validate_id(id: i64, name: &str) -> Result<(), String> {
        if id <= 0 {
            Err(format!("Invalid {}: {}", name, id))
        } else {
            Ok(())
        }
    }

    pub fn validate_not_empty(value: &str, name: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err(format!("{} cannot be empty", name))
        } else {
            Ok(())
        }
    }
}

pub fn serialize_to_json<T: serde::Serialize>(value: &T, context: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("{} serialization failed: {}", context, e))
}

pub fn serialize_to_value<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(value).map_err(|e| format!("{} serialization failed: {}", context, e))
}
