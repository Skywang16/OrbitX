/*!
 * ReactHandler implementation - AgentRunExecutor as ReAct handler
 *
 */

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::agents::{visible_task_profiles, AgentConfigLoader};
use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{AgentRunContext, AgentToolCallResult};
use crate::agent::core::executor::{AgentRunExecutor, ReactHandler};
use crate::agent::core::utils::should_render_tool_block;
use crate::agent::error::AgentRunResult;
use crate::agent::tools::{
    self, ToolDescriptionContext, ToolRegistry, ToolResultContent, ToolResultStatus,
};
use crate::agent::types::{Block, ToolBlock, ToolOutput, ToolStatus};
use crate::llm::anthropic_types::{CreateMessageRequest, ThinkingConfig};

const TOOL_OUTPUT_PREVIEW_MAX_CHARS: usize = 8000;
const DEFAULT_MAX_TOKENS: u32 = 32_768;

#[async_trait::async_trait]
impl ReactHandler for AgentRunExecutor {
    #[inline]
    async fn build_llm_request(
        &self,
        context: &AgentRunContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> AgentRunResult<CreateMessageRequest> {
        use crate::storage::repositories::AIModels;

        let model_config = AIModels::new(&self.inner.database)
            .find_by_id(model_id)
            .await?
            .ok_or_else(|| {
                crate::agent::error::AgentRunError::ConfigurationError(format!(
                    "Model not found: {model_id}"
                ))
            })?;

        let options = model_config.options.as_ref();
        let max_tokens = explicit_max_tokens(options).unwrap_or(DEFAULT_MAX_TOKENS);

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

        // Deep Thinking configuration (Anthropic Extended Thinking)
        let enable_deep_thinking = options
            .and_then(|opts| opts.get("enableDeepThinking"))
            .and_then(|v| v.as_bool())
            .is_some_and(|enabled| enabled);

        let thinking = if enable_deep_thinking {
            Some(ThinkingConfig::default_budget())
        } else {
            None
        };

        // When thinking is enabled, temperature must be 1.0
        let temperature = if thinking.is_some() {
            Some(1.0)
        } else {
            temperature
        };

        let tool_schemas = tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
            cwd: cwd.to_string(),
            agent_type: Some(context.agent_type.to_string()),
            allowed_subagent_types: load_allowed_task_profiles(cwd, context.agent_type.as_ref())
                .await,
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
        let developer_context = context.get_developer_context().await;
        let final_messages = if let Some(msgs) = messages {
            msgs
        } else {
            context.get_messages().await
        };

        Ok(CreateMessageRequest {
            model: model_id.to_string(),
            max_tokens,
            system: system_prompt,
            developer_context: (!developer_context.is_empty()).then_some(developer_context),
            messages: final_messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            stream: true,
            temperature,
            top_p,
            top_k,
            metadata: None,
            stop_sequences: None,
            thinking,
        })
    }

    #[inline]
    async fn execute_tools(
        &self,
        context: &AgentRunContext,
        _iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> AgentRunResult<Vec<AgentToolCallResult>> {
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
                    .assistant_upsert_block(Block::Tool(ToolBlock {
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

        // Convert to ToolCall and execute in parallel
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

        // Convert results and send events
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
            let duration_ms = match resp.result.execution_time_ms {
                Some(duration_ms) => duration_ms as i64,
                None => finished_at
                    .signed_duration_since(started_at)
                    .num_milliseconds()
                    .max(0),
            };

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

            results.push(AgentToolCallResult {
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
    async fn get_context_builder(&self, context: &AgentRunContext) -> Arc<ContextBuilder> {
        let file_tracker = context.file_tracker();
        Arc::new(ContextBuilder::new(file_tracker))
    }
}

fn explicit_max_tokens(options: Option<&Value>) -> Option<u32> {
    let max_tokens = options
        .and_then(|opts| opts.get("maxTokens"))
        .and_then(|value| value.as_i64())
        .and_then(|value| (value > 0).then_some(value as u32));
    if max_tokens.is_some() {
        return max_tokens;
    }

    options
        .and_then(|opts| opts.get("maxContextTokens"))
        .and_then(|value| value.as_u64())
        .map(|value| value.min(DEFAULT_MAX_TOKENS as u64) as u32)
}

async fn load_allowed_task_profiles(cwd: &str, agent_type: &str) -> Vec<String> {
    let workspace_root = PathBuf::from(cwd);
    let Ok(configs) = AgentConfigLoader::load_for_workspace(&workspace_root).await else {
        tracing::warn!(
            "Failed to load agent configs for workspace {}, cannot resolve task profiles for {}",
            workspace_root.display(),
            agent_type
        );
        return Vec::new();
    };
    let Some(caller) = configs.get(agent_type) else {
        tracing::warn!(
            "Agent config `{}` not found in workspace {}, no task profiles exposed",
            agent_type,
            workspace_root.display()
        );
        return Vec::new();
    };
    visible_task_profiles(caller, &configs)
}

/// Convert ToolResult to (status, json_value)
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
            // Return string directly, don't wrap in object
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
