/*!
 * ReAct Handler Trait - defines interface between AgentRunExecutor and ReactOrchestrator
 *
 */

use serde_json::Value;
use std::sync::Arc;

use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{AgentRunContext, AgentToolCallResult};
use crate::agent::error::AgentRunResult;
use crate::agent::tools::ToolRegistry;
use crate::llm::anthropic_types::CreateMessageRequest;

/// ReAct executor interface
///
#[async_trait::async_trait]
pub trait ReactHandler {
    /// Build LLM request
    ///
    /// Note: uses references to avoid cloning
    async fn build_llm_request(
        &self,
        context: &AgentRunContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> AgentRunResult<CreateMessageRequest>;

    /// Execute tool calls
    ///
    /// Note: returns results instead of modifying state, more functional
    async fn execute_tools(
        &self,
        context: &AgentRunContext,
        iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> AgentRunResult<Vec<AgentToolCallResult>>;

    /// Get ContextBuilder
    ///
    /// Note: returns Arc to avoid cloning the builder itself
    async fn get_context_builder(&self, context: &AgentRunContext) -> Arc<ContextBuilder>;
}
