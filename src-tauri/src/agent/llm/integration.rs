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
        let messages = if let Some(m) = additional_messages {
            std::collections::VecDeque::from(m)
        } else {
            context.get_messages().await
        };
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
    /// 内部执行工具调用：返回完整 ToolResult（包含 ext_info）
    async fn execute_tool_internal(
        &self,
        tool_name: &str,
        args: &Value,
        context: &TaskContext,
    ) -> crate::agent::tools::ToolResult {
        let tool_registry = context.tool_registry();
        tool_registry
            .execute_tool(tool_name, context, args.clone())
            .await
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

        // 执行工具调用，获取完整 ToolResult
        let tool_result = self
            .execute_tool_internal(&tool_name, &tool_arguments, context)
            .await;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // 将 ToolResult 转换为简化的 JSON（给 LLM 用）
        let result_json = if tool_result.is_error {
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

        // 记录工具执行结果（使用ToolExecutionLogger）
        let log_text = serde_json::to_string(&result_json)
            .unwrap_or_else(|_| "Tool execution completed".to_string());

        if tool_result.is_error {
            if let Err(e) = logger.log_failure(&log_id, &log_text, execution_time).await {
                warn!("Failed to log tool failure: {}", e);
            }
        } else {
            if let Err(e) = logger
                .log_success(
                    &log_id,
                    &crate::agent::tools::ToolResult {
                        content: vec![crate::agent::tools::ToolResultContent::Success(
                            log_text.clone(),
                        )],
                        is_error: false,
                        execution_time_ms: Some(execution_time),
                        ext_info: tool_result.ext_info.clone(),
                    },
                    execution_time,
                )
                .await
            {
                warn!("Failed to log tool success: {}", e);
            }
        }

        // 发送结果事件（包含 ext_info 给前端 UI）
        context
            .send_progress(TaskProgressPayload::ToolResult(ToolResultPayload {
                task_id: context.task_id.clone(),
                iteration,
                tool_id: call_id.clone(),
                tool_name: tool_name.clone(),
                result: result_json.clone(),
                is_error: tool_result.is_error,
                timestamp: Utc::now(),
                ext_info: tool_result.ext_info.clone(),
            }))
            .await?;

        // 如果是错误，递增错误计数
        if tool_result.is_error {
            context.increment_error_count().await?;
        }

        // 创建 ToolCallResult（简化版给 LLM context）
        let tool_call_result = ToolCallResult {
            call_id: call_id.clone(),
            tool_name: tool_name.clone(),
            result: result_json.clone(),
            is_error: tool_result.is_error,
            execution_time_ms: execution_time,
        };

        // 6. 更新工具执行状态为已完成
        let completed_at = Some(chrono::Utc::now());
        let duration_ms = Some(execution_time as i64);

        if tool_call_result.is_error {
            // 从 result_json 中提取错误信息
            let error_msg = result_json
                .get("error")
                .and_then(|v| v.as_str())
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
            let result_str = serde_json::to_string(&result_json).unwrap_or_default();
            self.agent_persistence()
                .tool_executions()
                .update_status(
                    &call_id,
                    ToolExecutionStatus::Completed,
                    Some(&result_str),
                    None,
                    completed_at,
                    duration_ms,
                )
                .await?;
        }

        // 7. 添加工具结果到上下文
        context
            .add_tool_results(vec![tool_call_result.clone()])
            .await;

        Ok(tool_call_result)
    }

    /// 获取默认模型ID
    pub async fn get_default_model_id(&self) -> TaskExecutorResult<String> {
        // 从数据库中获取第一个可用模型
        let db = self.database();
        let models = crate::storage::repositories::AIModels::new(&db)
            .find_all()
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
