/*!
 * LLM集成模块 - TaskExecutor的LLM调用逻辑（已从 task_executor/llm_integration.rs 平移）
 */

use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::core::executor::TaskExecutor;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::{TaskProgressPayload, ToolResultPayload, ToolUsePayload};
use crate::agent::persistence::ToolExecutionStatus;
use crate::agent::tools::{ToolDescriptionContext, ToolRegistry};
use crate::llm::anthropic_types::{CreateMessageRequest, MessageParam, SystemPrompt, Tool};
use chrono::Utc;
use serde_json::Value;
use tracing::warn;

impl TaskExecutor {
    /// 构建 LLM 请求（零转换，只做组装）
    pub(crate) async fn build_llm_request(
        &self,
        context: &TaskContext,
        model_id: String,
        tool_registry: &ToolRegistry,
        cwd: &str,
        additional_messages: Option<Vec<MessageParam>>,
    ) -> TaskExecutorResult<CreateMessageRequest> {
        // 1) 直接获取 Anthropic 原生消息与 system prompt（或使用提供的消息覆盖）
        let messages: Vec<MessageParam> = if let Some(m) = additional_messages { m } else { context.get_messages().await };
        let system: Option<SystemPrompt> = context.get_system_prompt().await;

        // 2) 构建工具定义（根据当前工作目录上下文）
        let tool_schemas = tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
            cwd: cwd.to_string(),
        });
        let tools: Vec<Tool> = tool_schemas
            .into_iter()
            .map(|schema| Tool {
                name: schema.name,
                description: schema.description,
                input_schema: schema.parameters,
            })
            .collect();

        // 3) 组装 CreateMessageRequest（零转换）
        Ok(CreateMessageRequest {
            model: model_id,
            messages,
            system,
            tools: if tools.is_empty() { None } else { Some(tools) },
            max_tokens: 4096,
            temperature: Some(0.7),
            stream: true,
            stop_sequences: None,
            top_p: None,
            top_k: None,
            metadata: None,
        })
    }
}

impl TaskExecutor {
    /// 内部执行工具调用：将 ToolResult 转换为 JSON Value（保持与 LLM 交互的一致性）
    async fn execute_tool_internal(
        &self,
        tool_name: &str,
        args: &Value,
        context: &TaskContext,
    ) -> TaskExecutorResult<Value> {
        // 使用 ToolRegistry 执行工具（现在返回 ToolResult 而不是 Result）
        let tool_registry = context.tool_registry();
        let tool_result = tool_registry
            .execute_tool(tool_name, context, args.clone())
            .await;

        // 将 ToolResult 转换为统一的 JSON Value 格式: {"result": "..."} 或 {"error": "..."}
        let result_json = if tool_result.is_error {
            // 错误情况: {"error": "错误信息"}
            let error_msg = tool_result
                .content
                .first()
                .map(|c| match c {
                    crate::agent::tools::ToolResultContent::Error(msg) => msg.clone(),
                    crate::agent::tools::ToolResultContent::Success(msg) => msg.clone(),
                })
                .unwrap_or_else(|| "Unknown error".to_string());

            serde_json::json!({ "error": error_msg })
        } else {
            // 成功情况: {"result": "结果内容"}
            let result_text = tool_result
                .content
                .first()
                .map(|c| match c {
                    crate::agent::tools::ToolResultContent::Success(text) => text.clone(),
                    crate::agent::tools::ToolResultContent::Error(msg) => msg.clone(),
                })
                .unwrap_or_else(|| "No result".to_string());

            serde_json::json!({ "result": result_text })
        };

        Ok(result_json)
    }

    /// 执行工具调用
    pub(crate) async fn execute_tool_call(
        &self,
        context: &TaskContext,
        iteration: u32,
        tool_id: String,
        tool_name: String,
        tool_arguments: serde_json::Value,
    ) -> TaskExecutorResult<ToolCallResult> {
        let call_id = tool_id.clone();
        let tool_name = tool_name.clone();
        let start_time = std::time::Instant::now();

        // 记录工具执行开始（使用ToolExecutionLogger, 内部会创建 Running 记录）
        let logger = self.tool_logger();
        let log_id = logger
            .log_start(context, &call_id, &tool_name, &tool_arguments)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to log tool start: {}", e);
                format!("{}_{}", call_id, chrono::Utc::now().timestamp_millis())
            });

        // 发送 ToolUse 事件 - 表示工具开始执行
        context
            .send_progress(TaskProgressPayload::ToolUse(ToolUsePayload {
                task_id: context.task_id.clone(),
                iteration,
                tool_id: call_id.clone(),
                tool_name: tool_name.clone(),
                params: tool_arguments.clone(),
                timestamp: Utc::now(),
            }))
            .await?;

        // 执行工具调用（保持与既有 JSON 结果结构一致）
        let result = self
            .execute_tool_internal(&tool_name, &tool_arguments, context)
            .await;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // 处理执行结果
        let tool_result = match result {
            Ok(tool_output) => {
                // 记录工具执行成功（使用ToolExecutionLogger）
                // 将 JSON 对象转为字符串记录
                let log_text = serde_json::to_string(&tool_output)
                    .unwrap_or_else(|_| "Tool execution succeeded".to_string());

                if let Err(e) = logger
                    .log_success(
                        &log_id,
                        &crate::agent::tools::ToolResult {
                            content: vec![crate::agent::tools::ToolResultContent::Success(
                                log_text,
                            )],
                            is_error: false,
                            execution_time_ms: Some(execution_time),
                            ext_info: None,
                        },
                        execution_time,
                    )
                    .await
                {
                    warn!("Failed to log tool success: {}", e);
                }

                // 发送成功结果事件
                context
                    .send_progress(TaskProgressPayload::ToolResult(ToolResultPayload {
                        task_id: context.task_id.clone(),
                        iteration,
                        tool_id: call_id.clone(),
                        tool_name: tool_name.clone(),
                        result: tool_output.clone(),
                        is_error: false,
                        timestamp: Utc::now(),
                    }))
                    .await?;

                ToolCallResult {
                    call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    result: tool_output,
                    is_error: false,
                    execution_time_ms: execution_time,
                }
            }
            Err(error) => {
                // 记录工具执行失败（使用ToolExecutionLogger）
                if let Err(e) = logger
                    .log_failure(&log_id, &error.to_string(), execution_time)
                    .await
                {
                    warn!("Failed to log tool failure: {}", e);
                }

                // 发送错误结果事件
                let error_result = serde_json::json!({
                    "error": error.to_string(),
                    "tool_name": tool_name,
                    "call_id": call_id
                });

                context
                    .send_progress(TaskProgressPayload::ToolResult(ToolResultPayload {
                        task_id: context.task_id.clone(),
                        iteration,
                        tool_id: call_id.clone(),
                        tool_name: tool_name.clone(),
                        result: error_result.clone(),
                        is_error: true,
                        timestamp: Utc::now(),
                    }))
                    .await?;

                // 递增错误计数
                context.increment_error_count().await?;

                ToolCallResult {
                    call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    result: error_result,
                    is_error: true,
                    execution_time_ms: execution_time,
                }
            }
        };

        // 6. 更新工具执行状态为已完成
        let completed_at = Some(chrono::Utc::now());
        let duration_ms = Some(tool_result.execution_time_ms as i64);

        if tool_result.is_error {
            // 从 result 中提取错误信息
            let error_msg = tool_result
                .result
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();

            self.agent_persistence()
                .tool_executions()
                .update_status(
                    &call_id,
                    ToolExecutionStatus::Error,
                    None,
                    Some(&error_msg),
                    completed_at,
                    duration_ms,
                )
                .await?;
        } else {
            let result_json = serde_json::to_string(&tool_result.result).unwrap_or_default();
            self.agent_persistence()
                .tool_executions()
                .update_status(
                    &call_id,
                    ToolExecutionStatus::Completed,
                    Some(&result_json),
                    None,
                    completed_at,
                    duration_ms,
                )
                .await?;
        }

// 7. 添加工具结果到上下文
        context.add_tool_results(vec![tool_result.clone()]).await;

        Ok(tool_result)
    }

    /// 获取默认模型ID
    pub async fn get_default_model_id(&self) -> TaskExecutorResult<String> {
        // 从数据库中获取第一个可用模型
        let models = self
            .repositories()
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .map_err(TaskExecutorError::from)?;

        if let Some(any_model) = models.first() {
            return Ok(any_model.id.clone());
        }

        Err(TaskExecutorError::InternalError(
            "No available models found. Please add at least one model in Settings -> Models."
                .to_string(),
        ))
    }

    // react 解析器已在 agent::react 中实现
}
