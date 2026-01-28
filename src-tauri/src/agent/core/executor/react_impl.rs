/*!
 * ReactHandler 实现 - TaskExecutor作为ReAct处理器
 *
 */

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::core::executor::{ReactHandler, TaskExecutor};
use crate::agent::error::TaskExecutorResult;
use crate::agent::tools::{
    self, ToolDescriptionContext, ToolRegistry, ToolResultContent, ToolResultStatus,
};
use crate::agent::types::{Block, ToolBlock, ToolOutput, ToolStatus};
use crate::llm::anthropic_types::CreateMessageRequest;

const TOOL_OUTPUT_PREVIEW_MAX_CHARS: usize = 8000;

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
                    "Model not found: {model_id}"
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
        _iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> TaskExecutorResult<Vec<ToolCallResult>> {
        let mut tool_started_at: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();
        let mut tool_inputs: HashMap<String, Value> = HashMap::new();
        let mut rendered_tool_blocks: HashMap<String, bool> = HashMap::new();

        for (call_id, tool_name, params) in &tool_calls {
            let now = chrono::Utc::now();
            let mut params = params.clone();
            if tool_name == "task" {
                if let Value::Object(ref mut obj) = params {
                    obj.insert("call_id".to_string(), Value::String(call_id.clone()));
                }
            }
            tool_started_at.insert(call_id.clone(), now);
            tool_inputs.insert(call_id.clone(), params.clone());

            let render = should_render_tool_block(context, tool_name).await;
            rendered_tool_blocks.insert(call_id.clone(), render);
            if render {
                context
                    .assistant_append_block(Block::Tool(ToolBlock {
                        id: call_id.clone(),
                        call_id: call_id.clone(),
                        name: tool_name.clone(),
                        status: ToolStatus::Running,
                        input: params.clone(),
                        output: None,
                        compacted_at: None,
                        started_at: now,
                        finished_at: None,
                        duration_ms: None,
                    }))
                    .await?;
            }
        }

        // 转换为 ToolCall 并并行执行
        let calls: Vec<tools::ToolCall> = tool_calls
            .into_iter()
            .map(|(id, name, mut params)| {
                if name == "task" {
                    if let Value::Object(ref mut obj) = params {
                        obj.insert("call_id".to_string(), Value::String(id.clone()));
                    }
                }
                tools::ToolCall { id, name, params }
            })
            .collect();

        let responses = tools::execute_batch(&context.tool_registry(), context, calls).await;

        // 转换结果并发送事件
        let mut results = Vec::with_capacity(responses.len());
        for resp in responses {
            // These lookups must succeed - they were inserted in the loop above.
            // If they fail, it indicates a logic error in execute_batch.
            let Some(&started_at) = tool_started_at.get(&resp.id) else {
                tracing::error!("⚠️  Missing started_at for tool: {}", resp.id);
                continue;
            };
            let Some(input) = tool_inputs.get(&resp.id).cloned() else {
                tracing::error!("⚠️  Missing input for tool: {}", resp.id);
                continue;
            };
            let Some(&should_render) = rendered_tool_blocks.get(&resp.id) else {
                tracing::error!("⚠️  Missing render flag for tool: {}", resp.id);
                continue;
            };

            let (result_status, result_value) = convert_result(&resp.result);
            let preview_value =
                truncate_tool_output_value(&result_value, TOOL_OUTPUT_PREVIEW_MAX_CHARS);
            let finished_at = chrono::Utc::now();
            let duration_ms = resp
                .result
                .execution_time_ms
                .map(|v| v as i64)
                .unwrap_or_else(|| {
                    finished_at
                        .signed_duration_since(started_at)
                        .num_milliseconds()
                        .max(0)
                });

            let status = match result_status {
                ToolResultStatus::Success => ToolStatus::Completed,
                ToolResultStatus::Error => ToolStatus::Error,
                ToolResultStatus::Cancelled => ToolStatus::Cancelled,
            };

            if should_render {
                context
                    .assistant_update_block(
                        &resp.id,
                        Block::Tool(ToolBlock {
                            id: resp.id.clone(),
                            call_id: resp.id.clone(),
                            name: resp.name.clone(),
                            status,
                            input,
                            output: Some(ToolOutput {
                                content: preview_value.clone(),
                                title: None,
                                metadata: resp.result.ext_info.clone(),
                                cancel_reason: resp.result.cancel_reason.clone(),
                            }),
                            compacted_at: None,
                            started_at,
                            finished_at: Some(finished_at),
                            duration_ms: Some(duration_ms),
                        }),
                    )
                    .await?;
            }

            results.push(ToolCallResult {
                call_id: resp.id,
                tool_name: resp.name,
                result: result_value,
                status: result_status,
                execution_time_ms: duration_ms as u64,
            });
        }

        context.add_tool_results(results.clone()).await?;
        Ok(results)
    }

    #[inline]
    async fn get_context_builder(&self, context: &TaskContext) -> Arc<ContextBuilder> {
        let file_tracker = context.file_tracker();
        Arc::new(ContextBuilder::new(file_tracker))
    }
}

async fn should_render_tool_block(context: &TaskContext, tool_name: &str) -> bool {
    let Some(meta) = context.tool_registry().get_tool_metadata(tool_name).await else {
        return true;
    };
    !meta.tags.iter().any(|t| t == "ui:hidden")
}

/// 转换 ToolResult 到 (status, json_value)
#[inline]
fn convert_result(result: &tools::ToolResult) -> (ToolResultStatus, Value) {
    match result.status {
        ToolResultStatus::Success => {
            let content = result
                .content
                .iter()
                .filter_map(|c| match c {
                    ToolResultContent::Success(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            // 直接返回字符串，不要包装在对象里
            (ToolResultStatus::Success, serde_json::json!(content))
        }
        ToolResultStatus::Error => {
            let msg = result
                .content
                .iter()
                .filter_map(|c| match c {
                    ToolResultContent::Error(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (ToolResultStatus::Error, serde_json::json!(msg))
        }
        ToolResultStatus::Cancelled => {
            let msg = result
                .content
                .iter()
                .filter_map(|c| match c {
                    ToolResultContent::Error(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (ToolResultStatus::Cancelled, serde_json::json!(msg))
        }
    }
}

fn truncate_tool_output_value(value: &Value, max_chars: usize) -> Value {
    let mut out = value.clone();
    match &mut out {
        Value::String(s) => truncate_in_place(s, max_chars),
        Value::Object(map) => {
            for key in ["result", "error", "cancelled"] {
                if let Some(Value::String(s)) = map.get_mut(key) {
                    truncate_in_place(s, max_chars);
                }
            }
        }
        _ => {}
    }
    out
}

fn truncate_in_place(s: &mut String, max_chars: usize) {
    if max_chars == 0 {
        s.clear();
        return;
    }
    if s.chars().count() <= max_chars {
        return;
    }
    let truncated: String = s.chars().take(max_chars).collect();
    *s = format!("{truncated}... (truncated)");
}
