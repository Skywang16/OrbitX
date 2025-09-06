/*!
 * 错误处理迁移适配器
 *
 * 这是一个临时模块，用于在重构过程中提供从旧的错误系统到统一 anyhow 系统的转换。
 * 重构完成后，这个文件将被删除。
 */

use crate::ai::tool::ast::types::AstError;
use crate::llm::types::LLMError;
use crate::mux::error::TerminalError;
use crate::terminal::types::ContextError;
use anyhow::Error as AnyhowError;

// ============================================================================
// 错误转换适配器和便捷函数
// ============================================================================
//
// 注意：由于 anyhow 已经为所有实现了 std::error::Error 的类型提供了
// 通用的 From<E> 实现，而我们的错误类型（使用 thiserror）已经自动
// 实现了 std::error::Error，所以不需要手动实现 From trait。
//
// 这里提供的是便捷的转换函数，用于在需要时添加额外的上下文信息。

/// TerminalError 到 anyhow::Error 的便捷转换函数
pub fn convert_terminal_error(err: TerminalError) -> AnyhowError {
    // TerminalError 已经实现了 std::error::Error，可以直接转换为 anyhow::Error
    // 这里我们使用 anyhow::Error::from() 进行转换，然后可以选择性地添加上下文
    AnyhowError::from(err)
}

/// ContextError 到 anyhow::Error 的便捷转换函数
pub fn convert_context_error(err: ContextError) -> AnyhowError {
    match err {
        ContextError::Internal { source } => {
            // 内部错误已经是 anyhow::Error，直接返回
            source
        }
        other => {
            // 其他错误类型可以直接转换
            AnyhowError::from(other)
        }
    }
}

/// LLMError 到 anyhow::Error 的便捷转换函数
pub fn convert_llm_error(err: LLMError) -> AnyhowError {
    AnyhowError::from(err)
}

/// AstError 到 anyhow::Error 的便捷转换函数
pub fn convert_ast_error(err: AstError) -> AnyhowError {
    AnyhowError::from(err)
}

// ============================================================================
// 向后兼容的便捷转换函数
// ============================================================================

/// 将 TerminalError 转换为 anyhow::Error 并添加上下文
pub fn terminal_error_with_context(err: TerminalError, context: &str) -> AnyhowError {
    AnyhowError::from(err).context(context.to_string())
}

/// 将 ContextError 转换为 anyhow::Error 并添加上下文
pub fn context_error_with_context(err: ContextError, context: &str) -> AnyhowError {
    AnyhowError::from(err).context(context.to_string())
}

/// 将 LLMError 转换为 anyhow::Error 并添加上下文
pub fn llm_error_with_context(err: LLMError, context: &str) -> AnyhowError {
    AnyhowError::from(err).context(context.to_string())
}

/// 将 AstError 转换为 anyhow::Error 并添加上下文
pub fn ast_error_with_context(err: AstError, context: &str) -> AnyhowError {
    AnyhowError::from(err).context(context.to_string())
}

// ============================================================================
// 统一的错误日志记录机制
// ============================================================================

/// 记录错误并返回 anyhow::Error
pub fn log_and_return_error(err: impl Into<AnyhowError>, operation: &str) -> AnyhowError {
    let error = err.into();
    tracing::error!("操作失败 [{}]: {}", operation, error);
    error
}

/// 为 anyhow::Error 添加错误日志记录的扩展方法
pub trait LogError {
    fn log_error(self, operation: &str) -> Self;
}

impl LogError for AnyhowError {
    fn log_error(self, operation: &str) -> Self {
        tracing::error!("操作失败 [{}]: {}", operation, self);
        self
    }
}

// ============================================================================
// 迁移验证工具
// ============================================================================

/// 验证错误转换的正确性
#[cfg(test)]
mod migration_tests {
    use super::*;
    use crate::mux::PaneId;

    #[test]
    fn test_terminal_error_conversion() {
        let terminal_err = TerminalError::config("测试错误");
        let anyhow_err = convert_terminal_error(terminal_err);
        assert!(anyhow_err.to_string().contains("配置错误: 测试错误"));
    }

    #[test]
    fn test_context_error_conversion() {
        let context_err = ContextError::PaneNotFound {
            pane_id: PaneId::new(123),
        };
        let anyhow_err = convert_context_error(context_err);
        assert!(anyhow_err.to_string().contains("面板不存在: 123"));
    }

    #[test]
    fn test_llm_error_conversion() {
        let llm_err = LLMError::ModelNotFound("gpt-4".to_string());
        let anyhow_err = convert_llm_error(llm_err);
        assert!(anyhow_err.to_string().contains("LLM模型未找到: gpt-4"));
    }

    #[test]
    fn test_ast_error_conversion() {
        let ast_err = AstError::FileNotFound("/path/to/file.rs".to_string());
        let anyhow_err = convert_ast_error(ast_err);
        assert!(anyhow_err
            .to_string()
            .contains("AST解析：文件不存在: /path/to/file.rs"));
    }

    #[test]
    fn test_automatic_conversion() {
        // 测试自动的 From 转换（由 anyhow 提供）
        let terminal_err = TerminalError::config("自动转换测试");
        let anyhow_err: AnyhowError = terminal_err.into();
        assert!(anyhow_err.to_string().contains("配置错误: 自动转换测试"));
    }

    #[test]
    fn test_error_with_context() {
        let terminal_err = TerminalError::config("测试错误");
        let anyhow_err = terminal_error_with_context(terminal_err, "执行配置加载");
        assert!(anyhow_err.to_string().contains("执行配置加载"));
    }
}

#[cfg(test)]
mod test_utils {
    use super::*;

    /// 创建测试用的 TerminalError
    pub fn create_test_terminal_error() -> TerminalError {
        TerminalError::config("测试配置错误")
    }

    /// 创建测试用的 ContextError
    pub fn create_test_context_error() -> ContextError {
        ContextError::NoActivePane
    }

    /// 创建测试用的 LLMError
    pub fn create_test_llm_error() -> LLMError {
        LLMError::Provider("测试提供商错误".to_string())
    }

    /// 创建测试用的 AstError
    pub fn create_test_ast_error() -> AstError {
        AstError::ParseError("测试解析错误".to_string())
    }
}
