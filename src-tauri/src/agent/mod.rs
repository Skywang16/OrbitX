/// Agent模块 - 提供完整的Agent系统
pub mod config;
pub mod error;
pub mod prompt;
pub mod types;

pub mod common; // 公共工具与模板等
pub mod compaction; // 上下文工程：Prune/Compact/断点加载
pub mod context; // 会话上下文追踪器与摘要器（阶段二）
pub mod core; // 执行器核心（仅执行器，不含工具相关）
pub mod llm; // LLM 集成与解析
pub mod mcp; // MCP 适配（预留）
pub mod persistence; // 持久化与仓库抽象
pub mod react; // ReAct 策略与解析(预留)
pub mod shell; // Shell 执行模块
pub mod state; // 任务上下文与错误
pub mod tools; // 工具接口与内置工具
pub mod utils; // 工具函数
pub use config::*;
pub use error::*;
pub use types::*;

pub use core::TaskExecutor;
pub use tools::{ToolExecutionLogger, ToolRegistry};

/// Agent服务，提供提示词构建功能
pub struct AgentService;

impl AgentService {
    pub async fn new() -> AgentResult<Self> {
        Ok(Self)
    }

    /// 构建Agent系统提示词
    pub async fn build_agent_system_prompt(
        &self,
        agent_name: &str,
        user_prompt: &str,
        tools: Vec<types::ToolSchema>,
        ext_sys_prompt: Option<String>,
    ) -> AgentResult<String> {
        let agent = types::Agent {
            name: agent_name.to_string(),
            description: "AI assistant specialized for terminal emulator applications".into(),
            capabilities: vec![],
            tools: tools.iter().map(|t| t.name.clone()).collect(),
        };

        let task = types::Task {
            id: "current".to_string(),
            session_id: 0,
            user_prompt: user_prompt.to_string(),
            xml: None,
            status: types::TaskStatus::Running,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let context = types::Context::default();

        prompt::build_agent_system_prompt(agent, Some(task), Some(context), tools, ext_sys_prompt)
            .await
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self
    }
}
