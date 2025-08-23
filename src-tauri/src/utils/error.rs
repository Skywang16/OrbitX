/*!
 * 错误处理模块
 *
 * 基于 anyhow 的统一错误处理系统，遵循 Rust 应用程序最佳实践。
 * 提供简洁、一致的错误处理接口，通过 context 提供丰富的错误信息。
 */

use anyhow::{anyhow, Result as AnyhowResult};

/// 统一的应用程序结果类型
///
/// 基于 anyhow::Result 的类型别名，提供统一的错误处理接口。
/// 所有模块都应该使用这个类型来返回可能失败的操作结果。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::AppResult;
///
/// fn some_operation() -> AppResult<String> {
///     std::fs::read_to_string("config.toml")
///         .context("读取配置文件失败")?;
///     Ok("success".to_string())
/// }
/// ```
pub type AppResult<T> = AnyhowResult<T>;

/// 统一的应用程序错误类型
///
/// 基于 anyhow::Error 的类型别名，提供统一的错误类型。
/// 通过 anyhow 的 context 机制提供丰富的错误信息。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::AppError;
/// use anyhow::anyhow;
///
/// let error: AppError = anyhow!("操作失败");
/// ```
pub type AppError = anyhow::Error;

/// Tauri 命令专用的结果类型
///
/// 为了兼容 Tauri 的命令系统，提供一个返回 String 错误的结果类型。
/// 在 Tauri 命令中使用这个类型，而在内部逻辑中使用 AppResult。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::TauriResult;
///
/// #[tauri::command]
/// async fn some_command() -> TauriResult<String> {
///     let result = do_internal_work()?; // 使用 AppResult
///     Ok(result)
/// }
/// ```
pub type TauriResult<T> = Result<T, String>;

// ============================================================================
// 便捷的错误处理工具函数
// ============================================================================

/// 创建简单的应用程序错误
///
/// 使用 anyhow! 宏创建包含指定消息的错误。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::app_error;
///
/// let error = app_error("操作失败");
/// ```
pub fn app_error(msg: impl Into<String>) -> AppError {
    anyhow!(msg.into())
}

/// 创建带上下文的错误转换函数
///
/// 返回一个闭包，可以将任何实现了 Display + Debug + Send + Sync 的错误
/// 转换为带有指定上下文信息的 AppError。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::app_error_with_context;
///
/// let result = std::fs::read_to_string("config.toml")
///     .map_err(app_error_with_context("读取配置文件失败"));
/// ```
pub fn app_error_with_context<T>(msg: &str) -> impl FnOnce(T) -> AppError + '_
where
    T: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    move |err| anyhow!("{}: {}", msg, err)
}

// ============================================================================
// Tauri 命令兼容性
// ============================================================================

/// 将 AppResult 转换为 Tauri 命令兼容的 Result<T, String>
///
/// Tauri 命令需要返回 Result<T, String> 类型，这个函数提供便捷的转换。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::{AppResult, to_tauri_result};
///
/// #[tauri::command]
/// async fn some_command() -> Result<String, String> {
///     let result: AppResult<String> = do_something();
///     to_tauri_result(result)
/// }
/// ```
pub fn to_tauri_result<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

/// 为 AppResult 提供便捷的转换方法
pub trait ToTauriResult<T> {
    fn to_tauri(self) -> Result<T, String>;
}

impl<T> ToTauriResult<T> for AppResult<T> {
    fn to_tauri(self) -> Result<T, String> {
        to_tauri_result(self)
    }
}

// ============================================================================
// 便捷的错误创建宏
// ============================================================================

/// 快速创建带上下文的错误
///
/// 提供便捷的宏来创建常见类型的错误，使用 anyhow 的功能。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::app_bail;
///
/// // 创建简单错误
/// app_bail!("操作失败");
///
/// // 创建带格式化的错误
/// app_bail!("文件 {} 不存在", file_path);
/// ```
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

/// 快速创建错误（不返回）
///
/// 类似于 app_bail! 但不会立即返回，而是创建错误值。
///
/// # 使用示例
/// ```rust
/// use crate::utils::error::app_error_msg;
///
/// let error = app_error_msg!("操作失败: {}", reason);
/// ```
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
// ============================================================================
// 统一的参数验证工具
// ============================================================================

/// 参数验证器
pub struct Validator;

impl Validator {
    /// 验证ID是否有效（大于0）
    pub fn validate_id(id: i64, name: &str) -> Result<(), String> {
        if id <= 0 {
            Err(format!("无效的{}: {}", name, id))
        } else {
            Ok(())
        }
    }

    /// 验证字符串不为空
    pub fn validate_not_empty(value: &str, name: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err(format!("{}不能为空", name))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// 序列化辅助工具
// ============================================================================

/// 序列化辅助函数
pub fn serialize_to_json<T: serde::Serialize>(value: &T, context: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("{}序列化失败: {}", context, e))
}

/// 序列化为JSON值的辅助函数
pub fn serialize_to_value<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(value).map_err(|e| format!("{}序列化失败: {}", context, e))
}
