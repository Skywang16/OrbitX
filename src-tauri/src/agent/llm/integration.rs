/*!
 * LLM集成模块 - TaskExecutor的LLM调用逻辑（已从 task_executor/llm_integration.rs 平移）
 */

use crate::agent::core::executor::TaskExecutor;
use crate::agent::events::{TaskProgressPayload, ToolResultPayload, ToolUsePayload};
use crate::agent::persistence::prelude::{ExecutionStepType, ToolCallStatus};
use crate::agent::state::context::{TaskContext, ToolCallResult};
use crate::agent::state::error::TaskExecutorResult;
use crate::llm::types::{LLMMessage, LLMRequest, LLMTool, LLMToolCall};
use chrono::Utc;
use serde_json::Value;
use tracing::warn;

impl TaskExecutor {
    /// 构建LLM请求
    pub(crate) async fn build_llm_request(
        &self,
        messages: Vec<LLMMessage>,
    ) -> TaskExecutorResult<LLMRequest> {
        // 获取默认模型配置
        let model_id = self.get_default_model_id().await?;

        // 构建工具定义
        let tools = self.build_tool_definitions().await?;

        Ok(LLMRequest {
            model: model_id,
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: None,
            stream: false,
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
        // 使用 ToolRegistry 执行工具
        let tool_result = self
            .tool_registry
            .execute_tool(tool_name, context, args.clone())
            .await?;

        // 将 ToolResult 转换为 JSON Value
        let result_json = if tool_result.is_error {
            serde_json::json!({
                "error": true,
                "message": tool_result
                    .content
                    .first()
                    .and_then(|c| match c {
                        crate::agent::tools::ToolResultContent::Error { message, .. } => Some(message.clone()),
                        crate::agent::tools::ToolResultContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "Unknown error".to_string())
            })
        } else {
            // 将工具结果内容转为 JSON
            let mut result = serde_json::json!({});

            for content in &tool_result.content {
                match content {
                    crate::agent::tools::ToolResultContent::Text { text } => {
                        result["text"] = serde_json::json!(text);
                    }
                    crate::agent::tools::ToolResultContent::Json { data } => {
                        result = data.clone();
                    }
                    crate::agent::tools::ToolResultContent::CommandOutput {
                        stdout,
                        stderr,
                        exit_code,
                    } => {
                        result["stdout"] = serde_json::json!(stdout);
                        result["stderr"] = serde_json::json!(stderr);
                        result["exit_code"] = serde_json::json!(exit_code);
                    }
                    crate::agent::tools::ToolResultContent::File { path } => {
                        result["file"] = serde_json::json!(path);
                    }
                    crate::agent::tools::ToolResultContent::Image { base64, format } => {
                        result["image_base64"] = serde_json::json!(base64);
                        result["image_format"] = serde_json::json!(format);
                    }
                    _ => {}
                }
            }

            result
        };

        Ok(result_json)
    }

    /// 执行工具调用
    pub(crate) async fn execute_tool_call(
        &self,
        context: &TaskContext,
        iteration: u32,
        tool_call: LLMToolCall,
    ) -> TaskExecutorResult<ToolCallResult> {
        let call_id = tool_call.id.clone();
        let tool_name = tool_call.name.clone();
        let start_time = std::time::Instant::now();

        // 1. 发送工具调用开始事件
        context
            .send_progress(TaskProgressPayload::ToolUse(ToolUsePayload {
                task_id: context.task_id.clone(),
                iteration,
                tool_id: call_id.clone(),
                tool_name: tool_name.clone(),
                params: tool_call.arguments.clone(),
                timestamp: Utc::now(),
            }))
            .await?;

        // 2. 记录工具执行开始（使用ToolExecutionLogger, 内部会创建 Running 记录）
        let log_id = self
            .tool_logger
            .log_start(context, &call_id, &tool_name, &tool_call.arguments)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to log tool start: {}", e);
                format!("{}_{}", call_id, chrono::Utc::now().timestamp_millis())
            });

        // 4. 执行工具调用（保持与旧实现一致的 JSON 结果）
        let result = self
            .execute_tool_internal(&tool_name, &tool_call.arguments, context)
            .await;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // 5. 处理执行结果
        let tool_result = match result {
            Ok(tool_output) => {
                // 记录工具执行成功（使用ToolExecutionLogger）
                if let Err(e) = self
                    .tool_logger
                    .log_success(
                        &log_id,
                        &crate::agent::tools::ToolResult {
                            content: vec![crate::agent::tools::ToolResultContent::Json {
                                data: tool_output.clone(),
                            }],
                            is_error: false,
                            execution_time_ms: Some(execution_time),
                            metadata: None,
                        },
                        execution_time,
                    )
                    .await
                {
                    warn!("Failed to log tool success: {}", e);
                }

                // 更新工具调用状态为完成
                self.repositories
                    .agent_tool_calls()
                    .update_status(
                        &call_id,
                        ToolCallStatus::Completed,
                        Some(tool_output.clone()),
                        None,
                    )
                    .await?;

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
                if let Err(e) = self
                    .tool_logger
                    .log_failure(&log_id, &error.to_string(), execution_time)
                    .await
                {
                    warn!("Failed to log tool failure: {}", e);
                }

                // 更新工具调用状态为错误
                self.repositories
                    .agent_tool_calls()
                    .update_status(
                        &call_id,
                        ToolCallStatus::Error,
                        None,
                        Some(error.to_string()),
                    )
                    .await?;

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

        // 6. 记录工具调用到执行日志
        self.log_execution_step(
            context,
            iteration,
            ExecutionStepType::ToolCall,
            serde_json::json!({
                "tool_call": {
                    "id": call_id,
                    "name": tool_name,
                    "arguments": tool_call.arguments
                }
            }),
        )
        .await?;

        self.log_execution_step(
            context,
            iteration,
            ExecutionStepType::ToolResult,
            serde_json::json!({
                "tool_result": {
                    "call_id": call_id,
                    "tool_name": tool_name,
                    "result": tool_result.result,
                    "is_error": tool_result.is_error,
                    "execution_time_ms": tool_result.execution_time_ms
                }
            }),
        )
        .await?;

        // 7. 添加工具结果到上下文
        context.add_tool_result(tool_result.clone()).await;
        context.save_context_snapshot().await?;

        Ok(tool_result)
    }

    /// 获取默认模型ID
    async fn get_default_model_id(&self) -> TaskExecutorResult<String> {
        // 优先从数据库中选择“已启用”的模型；找不到则退化到任意一个模型
        let models = self
            .repositories
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .map_err(|e| anyhow::anyhow!("获取可用模型失败: {}", e))?;

        if let Some(first_enabled) = models.iter().find(|m| m.enabled) {
            return Ok(first_enabled.id.clone());
        }

        if let Some(any_model) = models.first() {
            return Ok(any_model.id.clone());
        }

        Err(anyhow::anyhow!(
            "未找到任何可用模型，请在 设置 -> 模型 中添加并启用至少一个模型"
        ))
    }

    /// 构建工具定义
    async fn build_tool_definitions(&self) -> TaskExecutorResult<Vec<LLMTool>> {
        // 从工具注册表获取可用工具并转换为LLM工具定义
        let tool_schemas = self.tool_registry.get_tool_schemas().await;

        let tools: Vec<LLMTool> = tool_schemas
            .into_iter()
            .map(|schema| LLMTool {
                name: schema.name,
                description: schema.description,
                parameters: schema.parameters,
            })
            .collect();

        Ok(tools)
    }

    // react 解析器已在 agent::react 中实现
}
