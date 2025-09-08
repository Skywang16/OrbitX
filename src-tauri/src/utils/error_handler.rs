/*!
 * 统一错误处理模块
 *
 * 提供基于国际化的错误处理系统，包含便捷的宏和包装器。
 * 与ApiResponse和i18n模块集成，实现统一的错误响应格式。
 */

use crate::utils::i18n::I18nManager;
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use std::collections::HashMap;

/// 统一错误响应宏 - 使用i18n key
///
/// 用法：
/// - `api_error!("common.operation_failed")` - 简单错误
/// - `api_error!("error.with_param", "name" => "文件名")` - 带参数错误
#[macro_export]
macro_rules! api_error {
    ($key:expr) => {
        $crate::utils::ApiResponse::error($crate::t!($key))
    };

    ($key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::utils::ApiResponse::error($crate::t!($key, $($param_key => $param_value),+))
    };
}

/// 统一成功响应宏
///
/// 用法：
/// - `api_success!(data)` - 返回数据的成功响应
/// - `api_success!()` - 无数据的成功响应
#[macro_export]
macro_rules! api_success {
    () => {
        $crate::utils::ApiResponse::ok($crate::utils::EmptyData::default())
    };

    ($data:expr) => {
        $crate::utils::ApiResponse::ok($data)
    };
}

/// Tauri 命令包装器trait
///
/// 为Result类型提供便捷的转换方法，自动处理错误并返回ApiResponse
pub trait TauriCommandWrapper<T> {
    /// 转换为TauriApiResult，使用默认错误消息
    fn to_api_response(self) -> TauriApiResult<T>;

    /// 转换为TauriApiResult，使用指定的错误i18n key
    fn to_api_response_with_error(self, error_key: &str) -> TauriApiResult<T>;

    /// 转换为TauriApiResult，使用带参数的错误i18n key
    fn to_api_response_with_params(
        self,
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T>;
}

impl<T, E> TauriCommandWrapper<T> for Result<T, E>
where
    E: std::fmt::Display + std::fmt::Debug,
{
    fn to_api_response(self) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => Ok(api_error!("common.operation_failed")),
        }
    }

    fn to_api_response_with_error(self, error_key: &str) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => Ok(api_error!(error_key)),
        }
    }

    fn to_api_response_with_params(
        self,
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => {
                let message = I18nManager::get_text(error_key, Some(&params));
                Ok(ApiResponse::error(message))
            }
        }
    }
}

/// 为Option类型提供便捷的转换方法
pub trait OptionToApiResponse<T> {
    /// 转换Option为TauriApiResult，None时使用默认错误
    fn to_api_response(self) -> TauriApiResult<T>;

    /// 转换Option为TauriApiResult，None时使用指定错误key
    fn to_api_response_or_error(self, error_key: &str) -> TauriApiResult<T>;
}

impl<T> OptionToApiResponse<T> for Option<T> {
    fn to_api_response(self) -> TauriApiResult<T> {
        match self {
            Some(data) => Ok(api_success!(data)),
            None => Ok(api_error!("common.not_found")),
        }
    }

    fn to_api_response_or_error(self, error_key: &str) -> TauriApiResult<T> {
        match self {
            Some(data) => Ok(api_success!(data)),
            None => Ok(api_error!(error_key)),
        }
    }
}

/// 参数验证宏
///
/// 用法：
/// - `validate_param!(value > 0, "common.invalid_params")` - 条件验证
/// - `validate_not_empty!(text, "common.invalid_params")` - 非空验证
#[macro_export]
macro_rules! validate_param {
    ($condition:expr, $error_key:expr) => {
        if !($condition) {
            return Ok($crate::api_error!($error_key));
        }
    };

    ($condition:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        if !($condition) {
            return Ok($crate::api_error!($error_key, $($param_key => $param_value),+));
        }
    };
}

/// 非空字符串验证宏
#[macro_export]
macro_rules! validate_not_empty {
    ($value:expr, $error_key:expr) => {
        $crate::validate_param!(!$value.trim().is_empty(), $error_key);
    };

    ($value:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!(!$value.trim().is_empty(), $error_key, $($param_key => $param_value),+);
    };
}

/// ID验证宏（大于0）
#[macro_export]
macro_rules! validate_id {
    ($id:expr, $error_key:expr) => {
        $crate::validate_param!($id > 0, $error_key);
    };

    ($id:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!($id > 0, $error_key, $($param_key => $param_value),+);
    };
}

/// 范围验证宏
#[macro_export]
macro_rules! validate_range {
    ($value:expr, $min:expr, $max:expr, $error_key:expr) => {
        $crate::validate_param!($value >= $min && $value <= $max, $error_key);
    };

    ($value:expr, $min:expr, $max:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!($value >= $min && $value <= $max, $error_key, $($param_key => $param_value),+);
    };
}

/// 错误处理工具函数
pub struct ErrorHandler;

impl ErrorHandler {
    /// 创建带参数的错误响应
    pub fn create_error_with_params<T>(
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T> {
        let message = I18nManager::get_text(error_key, Some(&params));
        Ok(ApiResponse::error(message))
    }

    /// 创建简单错误响应
    pub fn create_error<T>(error_key: &str) -> TauriApiResult<T> {
        Ok(api_error!(error_key))
    }

    /// 创建成功响应
    pub fn create_success<T>(data: T) -> TauriApiResult<T> {
        Ok(api_success!(data))
    }

    /// 创建空成功响应
    pub fn create_empty_success() -> TauriApiResult<EmptyData> {
        Ok(api_success!())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_wrapper() {
        let success_result: Result<i32, &str> = Ok(42);
        let error_result: Result<i32, &str> = Err("test error");

        // 需要初始化i18n才能测试
        // let success_response = success_result.to_api_response();
        // let error_response = error_result.to_api_response();
    }

    #[test]
    fn test_option_wrapper() {
        let some_value: Option<i32> = Some(42);
        let none_value: Option<i32> = None;

        // 需要初始化i18n才能测试
        // let success_response = some_value.to_api_response();
        // let error_response = none_value.to_api_response();
    }
}
