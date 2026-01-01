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
use crate::agent::tools::{self, ToolDescriptionContext, ToolRegistry, ToolResultContent};
use crate::llm::anthropic_types::CreateMessageRequest;

#[async_trait::async_trait]
impl ReactHandler for TaskExecutor {
    #[inline]
    async fn build_llm_request(
        &self,
        context: &TaskContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> TaskExecutorResult<CreateMessageRequest> {
        use crate::storage::repositories::AIModels;

        let model_config = AIModels::new(&self.inner.database)
            .find_by_id(model_id)
            .await?
            .ok_or_else(|| {
                crate::agent::error::TaskExecutorError::ConfigurationError(format!(
                    "Model not found: {}",
                    model_id
                ))
            })?;

        let max_tokens = model_config
            .options
            .as_ref()
            .and_then(|opts| opts.get("maxTokens"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(4096);

        let temperature = model_config
            .options
            .as_ref()
            .and_then(|opts| opts.get("temperature"))
            .and_then(|v| v.as_f64())
            .or(Some(0.7));

        let top_p = model_config
            .options
            .as_ref()
            .and_then(|opts| opts.get("topP"))
            .and_then(|v| v.as_f64());

        let top_k = model_config
            .options
            .as_ref()
            .and_then(|opts| opts.get("topK"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let tool_schemas = tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
            cwd: cwd.to_string(),
        });

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
            max_tokens,
            system: system_prompt,
            messages: final_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            stream: true,
            temperature,
            top_p,
            top_k,
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
        // 发送所有 ToolUse 事件
        for (call_id, tool_name, params) in &tool_calls {
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
        }

        // 转换为 ToolCall 并并行执行
        let calls: Vec<tools::ToolCall> = tool_calls
            .into_iter()
            .map(|(id, name, params)| tools::ToolCall { id, name, params })
            .collect();

        let responses = tools::execute_batch(&context.tool_registry(), context, calls).await;

        // 转换结果并发送事件
        let mut results = Vec::with_capacity(responses.len());
        for resp in responses {
            let (is_error, result_value) = convert_result(&resp.result);

            context
                .send_progress(TaskProgressPayload::ToolResult(ToolResultPayload {
                    task_id: context.task_id.to_string(),
                    iteration,
                    tool_id: resp.id.clone(),
                    tool_name: resp.name.clone(),
                    result: result_value.clone(),
                    is_error,
                    ext_info: resp.result.ext_info.clone(),
                    timestamp: chrono::Utc::now(),
                }))
                .await?;

            results.push(ToolCallResult {
                call_id: resp.id,
                tool_name: resp.name,
                result: result_value,
                is_error,
                execution_time_ms: resp.result.execution_time_ms.unwrap_or(0),
            });
        }

        context.add_tool_results(results.clone()).await;
        Ok(results)
    }

    #[inline]
    async fn get_context_builder(&self, context: &TaskContext) -> Arc<ContextBuilder> {
        let file_tracker = context.file_tracker();
        Arc::new(ContextBuilder::new(file_tracker))
    }
}

/// 转换 ToolResult 到 (is_error, json_value)
#[inline]
fn convert_result(result: &tools::ToolResult) -> (bool, Value) {
    if result.is_error {
        let msg = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolResultContent::Error(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        (true, serde_json::json!({"error": msg}))
    } else {
        let content = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolResultContent::Success(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        (false, serde_json::json!({"result": content}))
    }
}
