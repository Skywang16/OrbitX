pub mod agents;
pub mod command_system;
/// Agent模块 - 提供完整的Agent系统
pub mod config;
pub mod error;
pub mod prompt;
pub mod skill;
pub mod types;

pub mod common; // 公共工具与模板等
pub mod compaction; // 上下文工程：Prune/Compact/断点加载
pub mod context; // 会话上下文追踪器与摘要器
pub mod core; // 执行器核心（仅执行器，不含工具相关）
pub mod mcp; // MCP 适配
pub mod permissions; // settings.json permissions (allow/deny/ask)
pub mod persistence; // 持久化与仓库抽象
pub mod react; // ReAct 策略与解析
pub mod shell; // Shell 执行模块
pub mod terminal; // Agent terminal subsystem
pub mod state; // 任务上下文与错误
pub mod tools; // 工具接口与内置工具
pub mod utils; // 工具函数
pub mod workspace_changes; // 工作区变更账本（用户/外部变更注入）
pub use config::*;
pub use error::*;
pub use types::*;

pub use core::TaskExecutor;
pub use tools::{ToolExecutionLogger, ToolRegistry};
