/*!
 * 工具模块
 *
 * 提供应用程序中使用的通用工具和基础设施功能。
 * 包括错误处理、国际化、API响应格式等核心功能。
 */

/// 原有错误处理模块
pub mod error;

/// 统一API响应结构
pub mod api_response;

/// 语言管理模块
pub mod language;

/// 国际化(i18n)模块
pub mod i18n;

/// 统一错误处理模块
pub mod error_handler;

/// 语言设置命令
pub mod language_commands;

// 重新导出核心类型和功能，方便使用
pub use api_response::{ApiResponse, EmptyData, TauriApiResult};
pub use error_handler::{ErrorHandler, OptionToApiResponse, TauriCommandWrapper};
pub use language::{Language, LanguageManager};
pub use language_commands::LanguageInfo;
