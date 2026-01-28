//! Prompt module - 统一的提示词加载与构建
//!
//! 所有提示词存储在 `prompts/` 目录下的 md 文件中：
//! - `prompts/base/` - 基础提示词片段（role, rules, methodology）
//! - `prompts/agents/` - Agent 完整提示词（带 frontmatter 配置）
//! - `prompts/reminders/` - 运行时注入的提示
//! - `prompts/system/` - 系统级提示（compaction, summary 等）

mod builder;
mod loader;
pub mod orchestrator;

pub use builder::{PromptBuilder, SystemPromptParts};
pub use loader::{BuiltinPrompts, PromptLoader};
