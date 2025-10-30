/*!
 * ReactHandler 实现 - TaskExecutor作为ReAct处理器
 *
 * 零成本抽象：所有方法都会被内联，无运行时开销
 */

use serde_json::Value;
use std::sync::Arc;

use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::core::executor::{ReactHandler, TaskExecutor};
use crate::agent::error::TaskExecutorResult;
use crate::agent::events::{TaskProgressPayload, ToolResultPayload, ToolUsePayload};
use crate::agent::tools::{ToolDescriptionContext, ToolRegistry};
use crate::llm::anthropic_types::CreateMessageRequest;

#[async_trait::async_trait]
impl ReactHandler for TaskExecutor {
    #[inline] // 编译器内联，零开销
    async fn build_llm_request(
        &self,
        context: &TaskContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> TaskExecutorResult<CreateMessageRequest> {
        // 获取工具schema
        let tool_schemas = tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
            cwd: cwd.to_string(),
        });

        // 转换为Anthropic Tool类型
        let tools: Vec<crate::llm::anthropic_types::Tool> = tool_schemas
            .into_iter()
            .map(|schema| crate::llm::anthropic_types::Tool {
                name: schema.name,
                description: schema.description,
                input_schema: schema.parameters,
            })
            .collect();

        let system_prompt = context.get_system_prompt().await;
        let final_messages = if let Some(msgs) = messages {
            msgs
        } else {
            context.get_messages().await
        };

        Ok(CreateMessageRequest {
            model: model_id.to_string(),
            max_tokens: 4096,
            system: system_prompt,
            messages: final_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            stream: true,
            temperature: Some(0.7),
            top_p: None,
            top_k: None,
            metadata: None,
            stop_sequences: None,
        })
    }

    #[inline]
    async fn execute_tools(
        &self,
        context: &TaskContext,
        iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> TaskExecutorResult<Vec<ToolCallResult>> {
        let mut results = Vec::with_capacity(tool_calls.len());

        // 串行执行工具调用（保持原有逻辑）
        for (call_id, tool_name, params) in tool_calls {
            // 发送ToolUse事件
            context
                .send_progress(TaskProgressPayload::ToolUse(ToolUsePayload {
                    task_id: context.task_id.to_string(),
                    iteration,
                    tool_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    params: params.clone(),
                    timestamp: chrono::Utc::now(),
                }))
                .await?;

            let start = std::time::Instant::now();

            // 执行工具
            let tool_result = context
                .tool_registry()
                .execute_tool(&tool_name, context, params.clone())
                .await;

            let execution_time = start.elapsed().as_millis() as u64;

            // 转换ToolResult到我们的格式
            let (is_error, result_value) = if tool_result.is_error {
                let error_msg = tool_result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        crate::agent::tools::ToolResultContent::Error(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                (true, serde_json::json!({"error": error_msg}))
            } else {
                let success_content = tool_result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        crate::agent::tools::ToolResultContent::Success(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                (false, serde_json::json!({"result": success_content}))
            };

            // 发送ToolResult事件
            context
                .send_progress(TaskProgressPayload::ToolResult(ToolResultPayload {
                    task_id: context.task_id.to_string(),
                    iteration,
                    tool_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    result: result_value.clone(),
                    is_error,
                    ext_info: tool_result.ext_info.clone(),
                    timestamp: chrono::Utc::now(),
                }))
                .await?;

            results.push(ToolCallResult {
                call_id,
                tool_name,
                result: result_value,
                is_error,
                execution_time_ms: execution_time,
            });
        }

        // 添加到context
        context.add_tool_results(results.clone()).await;

        Ok(results)
    }

    #[inline]
    async fn get_context_builder(&self, context: &TaskContext) -> Arc<ContextBuilder> {
        let file_tracker = context.file_tracker();
        Arc::new(ContextBuilder::new(file_tracker))
    }
}
