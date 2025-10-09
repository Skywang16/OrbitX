//! 补全提供者模块
//!
//! 定义各种补全数据源的提供者

pub mod context_aware;
pub mod filesystem;
pub mod git;
pub mod history;
pub mod npm;
pub mod system_commands;

pub use context_aware::*;
pub use filesystem::*;
pub use git::*;
pub use history::*;
pub use npm::*;
pub use system_commands::*;

use crate::completion::types::{CompletionContext, CompletionItem};
use crate::completion::error::CompletionProviderResult;
use async_trait::async_trait;

/// 补全提供者trait
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &'static str;

    /// 检查是否应该为给定上下文提供补全
    fn should_provide(&self, context: &CompletionContext) -> bool;

    /// 提供补全建议
    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>>;

    /// 获取提供者优先级（数字越大优先级越高）
    fn priority(&self) -> i32 {
        0
    }

    /// 支持downcast的方法
    fn as_any(&self) -> &dyn std::any::Any;
}
