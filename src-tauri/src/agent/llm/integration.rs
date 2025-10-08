/*!
 * LLM集成模块 - TaskExecutor的LLM调用逻辑（已从 task_executor/llm_integration.rs 平移）
 */

use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::core::executor::TaskExecutor;
use crate::agent::events::{TaskProgressPayload, ToolResultPayload};
use crate::agent::persistence::ToolExecutionStatus;
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
        // 使用 ToolRegistry 执行工具（现在返回 ToolResult 而不是 Result）
        let tool_result = self
            .tool_registry()
            .execute_tool(tool_name, context, args.clone())
            .await;

        // 将 ToolResult 转换为 JSON Value
        let result_json = if tool_result.is_error {
            let mut error_map = serde_json::Map::new();
            let message = tool_result
                .content
                .first()
                .and_then(|c| match c {
                    crate::agent::tools::ToolResultContent::Error { message, .. } => {
                        Some(message.clone())
                    }
                    crate::agent::tools::ToolResultContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Unknown error".to_string());

            error_map.insert("error".to_string(), serde_json::Value::Bool(true));
            error_map.insert("message".to_string(), serde_json::Value::String(message));

            if let Some(ext_info) = tool_result.ext_info.clone() {
                error_map.insert("extInfo".to_string(), ext_info);
            }

            serde_json::Value::Object(error_map)
        } else {
            let mut result_map = serde_json::Map::new();

            for content in &tool_result.content {
                match content {
                    crate::agent::tools::ToolResultContent::Text { text } => {
                        result_map.insert("text".to_string(), serde_json::json!(text));
                    }
                    crate::agent::tools::ToolResultContent::Json { data } => match data {
                        serde_json::Value::Object(obj) => {
                            for (key, value) in obj.iter() {
                                result_map.insert(key.clone(), value.clone());
                            }
                        }
                        other => {
                            result_map.insert("data".to_string(), other.clone());
                        }
                    },
                    crate::agent::tools::ToolResultContent::CommandOutput {
                        stdout,
                        stderr,
                        exit_code,
                    } => {
                        // 成功：只返回输出；失败：只返回错误
                        if *exit_code == 0 && stderr.is_empty() {
                            result_map.insert("output".to_string(), serde_json::json!(stdout));
                        } else {
                            // 失败时优先返回stderr，如果没有则返回stdout
                            let error_msg = if !stderr.is_empty() {
                                stderr
                            } else if !stdout.is_empty() {
                                stdout
                            } else {
                                &format!("Command failed with exit code {}", exit_code)
                            };
                            result_map.insert("error".to_string(), serde_json::json!(error_msg));
                        }
                    }
                    crate::agent::tools::ToolResultContent::File { path } => {
                        result_map.insert("file".to_string(), serde_json::json!(path));
                    }
                    crate::agent::tools::ToolResultContent::Image { base64, format } => {
                        result_map.insert("image_base64".to_string(), serde_json::json!(base64));
                        result_map.insert("image_format".to_string(), serde_json::json!(format));
                    }
                    crate::agent::tools::ToolResultContent::Error { message, .. } => {
                        result_map.insert("error".to_string(), serde_json::json!(message));
                    }
                }
            }

            serde_json::Value::Object(result_map)
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

        // 记录工具执行开始（使用ToolExecutionLogger, 内部会创建 Running 记录）
        let logger = self.tool_logger();
        let log_id = logger
            .log_start(context, &call_id, &tool_name, &tool_call.arguments)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to log tool start: {}", e);
                format!("{}_{}", call_id, chrono::Utc::now().timestamp_millis())
            });

        // 执行工具调用（保持与既有 JSON 结果结构一致）
        let result = self
            .execute_tool_internal(&tool_name, &tool_call.arguments, context)
            .await;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // 处理执行结果
        let tool_result = match result {
            Ok(tool_output) => {
                // 记录工具执行成功（使用ToolExecutionLogger）
                if let Err(e) = logger
                    .log_success(
                        &log_id,
                        &crate::agent::tools::ToolResult {
                            content: vec![crate::agent::tools::ToolResultContent::Json {
                                data: tool_output.clone(),
                            }],
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
        context.add_tool_result(tool_result.clone()).await;

        Ok(tool_result)
    }

    /// 获取默认模型ID
    pub async fn get_default_model_id(&self) -> TaskExecutorResult<String> {
        // 优先从数据库中选择“已启用”的模型；找不到则退化到任意一个模型
        let models = self
            .repositories()
            .ai_models()
            .find_all_with_decrypted_keys()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get available models: {}", e))?;

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
        let tool_schemas = self.tool_registry().get_tool_schemas().await;

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
