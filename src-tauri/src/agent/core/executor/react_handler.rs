/*!
 * ReAct Handler Trait - 定义TaskExecutor与ReactOrchestrator的接口
 * 
 * 零成本抽象：
 * - 使用trait而不是闭包，编译期单态化
 * - 使用引用而不是克隆
 * - 让编译器内联所有调用
 */

use std::sync::Arc;
use serde_json::Value;

use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::error::TaskExecutorResult;
use crate::agent::tools::ToolRegistry;
use crate::llm::anthropic_types::CreateMessageRequest;

/// ReAct执行器接口
/// 
/// 零成本抽象：编译器会为每个实现类型生成特化代码
#[async_trait::async_trait]
pub trait ReactHandler {
    /// 构建LLM请求
    /// 
    /// 注意：使用引用避免克隆
    async fn build_llm_request(
        &self,
        context: &TaskContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> TaskExecutorResult<CreateMessageRequest>;

    /// 执行工具调用
    /// 
    /// 注意：返回结果而不是修改状态，更函数式
    async fn execute_tools(
        &self,
        context: &TaskContext,
        iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> TaskExecutorResult<Vec<ToolCallResult>>;

    /// 获取ContextBuilder
    /// 
    /// 注意：返回Arc，避免克隆builder本身
    async fn get_context_builder(
        &self,
        context: &TaskContext,
    ) -> Arc<ContextBuilder>;
}

