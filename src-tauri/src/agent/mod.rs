/// Agent模块 - 提供完整的Agent系统
pub mod config;
pub mod error;
pub mod prompt;
pub mod types;

pub mod common; // 公共工具与模板等
pub mod core; // 执行器核心（仅执行器，不含工具相关）
pub mod events; // 任务进度事件类型
pub mod llm; // LLM 集成与解析
pub mod mcp; // MCP 适配（预留）
pub mod memory; // 对话/上下文压缩与快照策略（预留）
pub mod persistence; // 持久化与仓库抽象
pub mod plan; // 规划器与任务树
pub mod react; // ReAct 策略与解析（预留）
pub mod state; // 任务上下文与错误
pub mod tools; // 工具接口与内置工具

use crate::agent::types::{Agent, Context, Task, TaskStatus, ToolSchema};
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
        tools: Vec<ToolSchema>,
        ext_sys_prompt: Option<String>,
    ) -> AgentResult<String> {
        let agent = Agent {
            name: agent_name.to_string(),
            description: "AI assistant specialized for terminal emulator applications".into(),
            capabilities: vec![],
            tools: tools.iter().map(|t| t.name.clone()).collect(),
        };

        let task = Task {
            id: "current".to_string(),
            conversation_id: 0,
            user_prompt: user_prompt.to_string(),
            xml: None,
            status: TaskStatus::Running,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let context = Context::default();

        prompt::build_agent_system_prompt(agent, Some(task), Some(context), tools, ext_sys_prompt)
            .await
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self
    }
}
