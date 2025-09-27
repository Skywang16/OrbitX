/*!
 * TaskExecutor - 核心任务执行器（已迁移至 agent/core）
 *
 * 负责Agent任务的完整执行流程：
 * - ReAct循环管理
 * - LLM调用和响应解析
 * - 工具执行调度
 * - 状态持久化
 * - 并发任务管理
 */

use crate::agent::config::TaskExecutionConfig;
use crate::agent::core::chain::ToolChain;
use crate::agent::events::{
    FinishPayload, TaskCancelledPayload, TaskCompletedPayload, TaskCreatedPayload,
    TaskErrorPayload, TaskPausedPayload, TaskProgressPayload, TaskResumedPayload,
    TaskStartedPayload, TextPayload, ThinkingPayload, ToolUsePayload,
};
use crate::agent::plan::{Planner, TreePlanner};
use crate::agent::common::xml::build_agent_xml_from_planned;
use crate::agent::memory::compress_messages;
use crate::agent::persistence::prelude::{
    AgentExecutionLog, AgentTask, AgentTaskStatus, ExecutionStepType, Repository, RepositoryManager,
};
use crate::agent::prompt::{build_agent_system_prompt, build_agent_user_prompt};
use crate::agent::react::types::FinishReason;
use crate::agent::state::context::{LLMResponseParsed, TaskContext, ToolCallResult};
use crate::agent::state::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::tools::{
    logger::ToolExecutionLogger, ToolRegistry, ToolResult as ToolOutcome, ToolResultContent,
};
use crate::agent::types::{Agent, Context as AgentContext, Task, ToolSchema};
use crate::llm::registry::LLMRegistry;
use chrono::Utc;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use uuid::Uuid;

/// 任务执行参数（与前端风格统一 camelCase）
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteTaskParams {
    pub conversation_id: i64,
    pub user_prompt: String,
    pub config_overrides: Option<serde_json::Value>,
    pub restore_task_id: Option<String>,
}

/// 串行任务树执行参数
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteTaskTreeParams {
    pub conversation_id: i64,
    pub user_prompt: String,
    /// 若为 false，则仅进行单次 plan 与执行（不生成任务树）
    #[serde(default = "default_true")]
    pub generate_tree: bool,
    pub config_overrides: Option<serde_json::Value>,
}

fn default_true() -> bool { true }

/// 将流式文本拆分为 (thinking, visible, has_open_thinking)
/// - thinking: 已闭合的 <thinking>...</thinking> 内容 + 最后一个未闭合的 <thinking> 部分
/// - visible: 去除已闭合的 thinking 块，并在存在未闭合 thinking 时移除其后的可见文本
/// - has_open_thinking: 当前是否处于一个未闭合的 thinking 块中
fn split_thinking_sections(raw: &str) -> (String, String, bool) {
    if raw.is_empty() {
        return (String::new(), String::new(), false);
    }

    // 收集所有闭合的 <thinking>...</thinking> 内容
    let re_closed = Regex::new(r"(?is)<thinking>(.*?)</thinking>").unwrap();
    let mut thinking_parts: Vec<String> = Vec::new();
    for cap in re_closed.captures_iter(raw) {
        if let Some(m) = cap.get(1) {
            thinking_parts.push(m.as_str().to_string());
        }
    }

    // 从原文中移除所有闭合的 thinking 块，得到 working 文本
    let working = re_closed.replace_all(raw, "").to_string();

    let mut has_open_thinking = false;
    let mut partial = String::new();
    let mut visible = working.clone();

    if let Some(last_idx) = working.rfind("<thinking") {
        // 查找最后一个 '<thinking' 之后是否有 '>'
        let tail = &working[last_idx..];
        if let Some(_gt_offset) = tail.find('>') {
            // 存在完整的开头标签，但（在 working 中）没有匹配的闭合标签 => 视为未闭合块
            has_open_thinking = true;
            // 尝试定位标准的 "<thinking>"（不带属性），找不到就使用 last_idx
            let open_tag_idx = working.rfind("<thinking>").unwrap_or(last_idx);
            let start_content = open_tag_idx + working[open_tag_idx..].find('>').unwrap_or(0) + 1;
            if start_content <= working.len() {
                visible = working[..open_tag_idx].to_string();
                partial = working[start_content..].to_string();
            }
        } else {
            // 连 '>' 都未出现，认为是未完成的起始标签：将其之后内容从可见文本中移除
            has_open_thinking = true;
            visible = working[..last_idx].to_string();
        }
    }

    let thinking = {
        if partial.trim().is_empty() {
            thinking_parts.join("\n").trim().to_string()
        } else {
            let mut v = thinking_parts;
            v.push(partial);
            v.join("\n").trim().to_string()
        }
    };

    (thinking, visible, has_open_thinking)
}

/// 任务摘要信息
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub task_id: String,
    pub conversation_id: i64,
    pub status: String,
    pub current_iteration: u32,
    pub error_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// TaskExecutor核心结构体
pub struct TaskExecutor {
    /// 数据存储管理器
    pub(crate) repositories: Arc<RepositoryManager>,

    /// LLM服务注册表
    pub(crate) llm_registry: Arc<LLMRegistry>,

    /// 工具注册表
    pub(crate) tool_registry: Arc<ToolRegistry>,

    /// 工具执行日志记录器
    pub(crate) tool_logger: Arc<ToolExecutionLogger>,

    /// 活动任务映射（task_id -> TaskContext）
    pub(crate) active_tasks: Arc<RwLock<HashMap<String, Arc<TaskContext>>>>,

    /// 默认执行配置
    pub(crate) default_config: TaskExecutionConfig,
}

impl TaskExecutor {
    /// 创建新的TaskExecutor
    pub fn new(
        repositories: Arc<RepositoryManager>,
        llm_registry: Arc<LLMRegistry>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        let tool_logger = Arc::new(ToolExecutionLogger::new(repositories.clone(), true));

        Self {
            repositories,
            llm_registry,
            tool_registry,
            tool_logger,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            default_config: TaskExecutionConfig::default(),
        }
    }

    /// 执行任务（主入口）
    pub async fn execute_task(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<()> {
        // 1. 创建或恢复任务上下文
        let task_context = if let Some(restore_task_id) = params.restore_task_id {
            // 恢复已有任务
            self.restore_task_context(restore_task_id, Some(progress_channel))
                .await?
        } else {
            // 创建新任务
            self.create_task_context(params, Some(progress_channel))
                .await?
        };

        let task_id = task_context.task_id.clone();
        let context = Arc::new(task_context);

        // 将任务标记为运行中
        context.set_status(AgentTaskStatus::Running).await?;

        // 2. 添加到活动任务列表
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id.clone(), context.clone());
        }

        // 3. 发送任务开始事件
        context
            .send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
                task_id: task_id.clone(),
                iteration: context.current_iteration().await,
            }))
            .await?;

        // 4. 在后台执行ReAct循环
        let executor = self.clone();
        tokio::spawn(async move {
            let result = executor.run_react_loop(context.clone()).await;

            // 5. 处理执行结果
            match result {
                Ok(_) => {
                    // 任务成功完成
                    if let Err(e) = context.set_status(AgentTaskStatus::Completed).await {
                        error!("Failed to update task status to completed: {}", e);
                    }

                    if let Err(e) = context
                        .send_progress(TaskProgressPayload::TaskCompleted(TaskCompletedPayload {
                            task_id: task_id.clone(),
                            final_iteration: context.current_iteration().await,
                            completion_reason: "Task completed successfully".to_string(),
                            timestamp: Utc::now(),
                        }))
                        .await
                    {
                        error!("Failed to send task completed event: {}", e);
                    }
                }
                Err(e) => {
                    // 任务执行出错
                    executor
                        .handle_task_error(&task_id, e, context.clone())
                        .await;
                }
            }

            // 6. 从活动任务列表中移除
            {
                let mut active_tasks = executor.active_tasks.write().await;
                active_tasks.remove(&task_id);
            }
        });

        Ok(())
    }

    /// 执行“Plan → (可选)Tree → 串行父节点子树”的流程
    pub async fn execute_task_tree(
        &self,
        params: ExecuteTaskTreeParams,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<()> {
        // 0) 先创建一个用于规划的根上下文
        let root_params = ExecuteTaskParams {
            conversation_id: params.conversation_id,
            user_prompt: params.user_prompt.clone(),
            config_overrides: params.config_overrides.clone(),
            restore_task_id: None,
        };
        let root_ctx = Arc::new(
            self.create_task_context(root_params, Some(progress_channel.clone()))
                .await?,
        );

        // 1) 先进行单次 plan，保存 TaskDetail（用于 UI）
        let planner = Planner::new(root_ctx.clone());
        if let Err(e) = planner.plan(&params.user_prompt, true).await {
            return Err(TaskExecutorError::InternalError(format!("Plan 失败: {}", e)).into());
        }

        // 2) 视情况执行树规划
        let planned_tree = if params.generate_tree {
            let tree_planner = TreePlanner::new(root_ctx.clone());
            match tree_planner.plan_tree(&params.user_prompt).await {
                Ok(tree) => Some(tree),
                Err(e) => {
                    // 若树规划失败，退化为单任务执行
                    tracing::warn!("Tree 规划失败，退化为单任务: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 3) 按父节点串行执行
        if let Some(tree) = planned_tree {
            // 取 Level-1 父任务组
            let parents = tree.subtasks.unwrap_or_default();
            let tool_schemas_full = self.tool_registry.get_tool_schemas().await;
            let simple_tool_schemas: Vec<ToolSchema> = tool_schemas_full
                .into_iter()
                .map(|s| ToolSchema { name: s.name, description: s.description, parameters: s.parameters })
                .collect();

            let mut prev_summary: Option<String> = None;

            for (idx, parent) in parents.into_iter().enumerate() {
                // 3.1) 为父节点创建独立上下文
                let parent_prompt = parent
                    .description
                    .clone()
                    .or(parent.name.clone())
                    .unwrap_or_else(|| format!("Phase-{}", idx + 1));

                let parent_params = ExecuteTaskParams {
                    conversation_id: root_ctx.conversation_id,
                    user_prompt: parent_prompt.clone(),
                    config_overrides: None,
                    restore_task_id: None,
                };
                let parent_ctx = Arc::new(
                    self.create_task_context(parent_params, Some(progress_channel.clone()))
                        .await?,
                );

                // 3.2) 将父节点的 planned 结构转为 agent xml，覆盖到 prompts 中
                if let Ok(agent_xml) = build_agent_xml_from_planned(&parent) {
                    let tool_names: Vec<String> = simple_tool_schemas.iter().map(|t| t.name.clone()).collect();
                    let agent_info = Agent {
                        name: "OrbitX Agent".to_string(),
                        description: "An AI coding assistant for OrbitX".to_string(),
                        capabilities: vec![],
                        tools: tool_names,
                    };
                    let task_for_prompt = Task {
                        id: parent_ctx.task_id.clone(),
                        conversation_id: parent_ctx.conversation_id,
                        user_prompt: parent_prompt.clone(),
                        xml: Some(agent_xml),
                        status: crate::agent::types::TaskStatus::Created,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };

                    let mut prompt_ctx = AgentContext::default();
                    prompt_ctx
                        .additional_context
                        .insert("taskPrompt".to_string(), serde_json::Value::String(parent_prompt.clone()));

                    let system_prompt = build_agent_system_prompt(
                        agent_info.clone(),
                        Some(task_for_prompt.clone()),
                        Some(prompt_ctx.clone()),
                        simple_tool_schemas.clone(),
                        None,
                    )
                    .await
                    .map_err(|e| TaskExecutorError::InternalError(format!("构建系统提示词失败: {}", e)))?;

                    let user_prompt = build_agent_user_prompt(
                        agent_info,
                        Some(task_for_prompt),
                        Some(prompt_ctx),
                        simple_tool_schemas.clone(),
                    )
                    .await
                    .map_err(|e| TaskExecutorError::InternalError(format!("构建用户提示词失败: {}", e)))?;

                    parent_ctx.set_initial_prompts(system_prompt, user_prompt).await?;
                }

                // 3.2.1) 如该父节点存在二级子任务，注入引导消息，要求按顺序串行完成这些子任务
                if let Some(children) = &parent.subtasks {
                    if !children.is_empty() {
                        let mut buf = String::from("Planned subtasks for this phase (execute sequentially, reuse the same context):\n");
                        for (i, child) in children.iter().enumerate() {
                            let name = child
                                .name
                                .as_deref()
                                .unwrap_or(child.description.as_deref().unwrap_or("Subtask"));
                            let desc = child
                                .description
                                .as_deref()
                                .unwrap_or("");
                            buf.push_str(&format!("{}. {}\n{}\n\n", i + 1, name, desc));
                        }
                        parent_ctx.push_system_message(buf).await;
                    }
                }

                // 3.3) 若有上一阶段总结，注入到上下文作为系统消息
                if let Some(summary) = prev_summary.take() {
                    parent_ctx
                        .push_system_message(format!("Previous phase summary:\n{}", summary))
                        .await;
                }

                // 3.4) 注册为活动任务并发送开始事件
                {
                    let mut active = self.active_tasks.write().await;
                    active.insert(parent_ctx.task_id.clone(), parent_ctx.clone());
                }
                parent_ctx
                    .set_status(AgentTaskStatus::Running)
                    .await?;
                parent_ctx
                    .send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
                        task_id: parent_ctx.task_id.clone(),
                        iteration: parent_ctx.current_iteration().await,
                    }))
                    .await?;

                // 3.5) 串行执行该父节点的 ReAct 循环
                if let Err(e) = self.run_react_loop(parent_ctx.clone()).await {
                    self.handle_task_error(&parent_ctx.task_id, e, parent_ctx.clone())
                        .await;
                } else {
                    // 标记完成
                    parent_ctx
                        .set_status(AgentTaskStatus::Completed)
                        .await
                        .ok();
                    parent_ctx
                        .send_progress(TaskProgressPayload::TaskCompleted(TaskCompletedPayload {
                            task_id: parent_ctx.task_id.clone(),
                            final_iteration: parent_ctx.current_iteration().await,
                            completion_reason: "Parent phase completed".to_string(),
                            timestamp: Utc::now(),
                        }))
                        .await
                        .ok();

                    // 3.6) 提取该父节点的最终可见回答作为阶段总结
                    let messages = parent_ctx.get_messages().await;
                    prev_summary = extract_last_assistant_text(&messages);
                }

                // 3.7) 从活动任务中移除
                {
                    let mut active = self.active_tasks.write().await;
                    active.remove(&parent_ctx.task_id);
                }
            }
        } else {
            // 无任务树，直接执行单任务
            let params_single = ExecuteTaskParams {
                conversation_id: root_ctx.conversation_id,
                user_prompt: params.user_prompt,
                config_overrides: None,
                restore_task_id: None,
            };
            self.execute_task(params_single, progress_channel).await?;
        }

        Ok(())
    }

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str) -> TaskExecutorResult<()> {
        let active_tasks = self.active_tasks.read().await;
        if let Some(context) = active_tasks.get(task_id) {
            context.set_status(AgentTaskStatus::Paused).await?;

            context
                .send_progress(TaskProgressPayload::TaskPaused(TaskPausedPayload {
                    task_id: task_id.to_string(),
                    reason: "User requested pause".to_string(),
                    timestamp: Utc::now(),
                }))
                .await?;
        } else {
            return Err(TaskExecutorError::TaskNotFound(task_id.to_string()).into());
        }

        Ok(())
    }

    /// 恢复任务
    pub async fn resume_task(
        &self,
        task_id: &str,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<()> {
        // 检查任务是否在活动列表中
        {
            let active_tasks = self.active_tasks.read().await;
            if active_tasks.contains_key(task_id) {
                return Err(TaskExecutorError::InternalError(
                    "Task is already running".to_string(),
                )
                .into());
            }
        }

        // 恢复任务上下文
        let task_context = self
            .restore_task_context(task_id.to_string(), Some(progress_channel))
            .await?;

        // 检查任务状态
        if task_context.status().await != AgentTaskStatus::Paused {
            return Err(TaskExecutorError::InvalidStateTransition {
                from: task_context.status().await.as_str().to_string(),
                to: "running".to_string(),
            }
            .into());
        }

        // 更新状态为运行中
        task_context.set_status(AgentTaskStatus::Running).await?;

        let task_id = task_context.task_id.clone();
        let context = Arc::new(task_context);

        // 添加到活动任务列表
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id.clone(), context.clone());
        }

        // 发送恢复事件
        context
            .send_progress(TaskProgressPayload::TaskResumed(TaskResumedPayload {
                task_id: task_id.clone(),
                from_iteration: context.current_iteration().await,
                timestamp: Utc::now(),
            }))
            .await?;

        // 继续执行ReAct循环
        let executor = self.clone();
        tokio::spawn(async move {
            let result = executor.run_react_loop(context.clone()).await;

            match result {
                Ok(_) => {
                    if let Err(e) = context.set_status(AgentTaskStatus::Completed).await {
                        error!("Failed to update task status: {}", e);
                    }
                }
                Err(e) => {
                    executor
                        .handle_task_error(&task_id, e, context.clone())
                        .await;
                }
            }

            // 从活动任务列表中移除
            {
                let mut active_tasks = executor.active_tasks.write().await;
                active_tasks.remove(&task_id);
            }
        });

        Ok(())
    }

    /// 取消任务
    pub async fn cancel_task(
        &self,
        task_id: &str,
        reason: Option<String>,
    ) -> TaskExecutorResult<()> {
        let active_tasks = self.active_tasks.read().await;
        if let Some(context) = active_tasks.get(task_id) {
            context.set_status(AgentTaskStatus::Cancelled).await?;

            context
                .send_progress(TaskProgressPayload::TaskCancelled(TaskCancelledPayload {
                    task_id: task_id.to_string(),
                    reason: reason.unwrap_or_else(|| "User cancelled".to_string()),
                    timestamp: Utc::now(),
                }))
                .await?;
        } else {
            return Err(TaskExecutorError::TaskNotFound(task_id.to_string()).into());
        }

        Ok(())
    }

    /// 列出任务
    pub async fn list_tasks(
        &self,
        conversation_id: Option<i64>,
        status_filter: Option<String>,
    ) -> TaskExecutorResult<Vec<TaskSummary>> {
        let tasks = if let Some(conv_id) = conversation_id {
            self.repositories
                .agent_tasks()
                .find_by_conversation_id(conv_id)
                .await?
        } else {
            self.repositories.agent_tasks().find_all().await?
        };

        let mut summaries = Vec::new();
        for task in tasks {
            if let Some(status) = &status_filter {
                if task.status.as_str() != status {
                    continue;
                }
            }

            summaries.push(TaskSummary {
                task_id: task.task_id,
                conversation_id: task.conversation_id,
                status: task.status.as_str().to_string(),
                current_iteration: task.current_iteration,
                error_count: task.error_count,
                created_at: task.created_at,
                updated_at: task.updated_at,
            });
        }

        Ok(summaries)
    }

    /// ReAct循环执行（核心逻辑）
    pub(crate) async fn run_react_loop(&self, context: Arc<TaskContext>) -> TaskExecutorResult<()> {
        info!("Starting ReAct loop for task: {}", context.task_id);

        while !context.should_stop().await {
            context.check_aborted(false).await?;

            let react_iteration_index = {
                let runtime_handle = context.react_runtime();
                let mut runtime = runtime_handle.write().await;
                runtime.start_iteration()
            };

            // 1. 递增迭代次数
            let iteration = context.increment_iteration().await?;
            {
                let state_handle = context.state_manager();
                let mut state = state_handle.write().await;
                state.reset_idle_rounds();
            }
            debug!("Task {} iteration {}", context.task_id, iteration);

            // 2. 构建LLM请求
            let raw_messages = context.get_messages().await;
            let messages = compress_messages(raw_messages, 200_000);
            let llm_request = self.build_llm_request(messages).await?;
            let llm_request_snapshot = llm_request.clone();

            // 3. 流式调用LLM，并增量推送文本
            let llm_service = crate::llm::service::LLMService::new(context.repositories().clone());
            let mut stream = llm_service
                .call_stream(llm_request)
                .await
                .map_err(|e| TaskExecutorError::InternalError(format!("LLM流式调用失败: {}", e)))?;

            let mut final_answer_acc = String::new();
            let mut pending_tool_calls: Vec<crate::llm::types::LLMToolCall> = Vec::new();
            let mut finished_with_tool_calls = false;
            let mut finish_reason_enum: Option<FinishReason> = None;

            // EKO 风格的思考/可见文本拆分与流式推送
            let mut stream_text = String::new();
            let mut saw_thinking_tag = false;
            let mut last_thinking_sent = String::new();
            let mut last_visible_sent = String::new();
            let mut announced_tool_ids: HashSet<String> = HashSet::new();
            let mut thinking_stream_id: Option<String> = None;
            let mut text_stream_id: Option<String> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => match chunk {
                        crate::llm::types::LLMStreamChunk::Delta {
                            content,
                            tool_calls,
                        } => {
                            if let Some(text) = content {
                                stream_text.push_str(&text);
                                final_answer_acc.push_str(&text);

                                if !saw_thinking_tag && stream_text.contains("<thinking") {
                                    saw_thinking_tag = true;
                                }

                                let (thinking, visible, has_open_thinking) =
                                    split_thinking_sections(&stream_text);
                                let thinking_trim = thinking.trim().to_string();
                                let can_send_visible = !visible.is_empty()
                                    && !visible.contains("<thinking")
                                    && !has_open_thinking
                                    && visible.trim().len() > 0;

                                if saw_thinking_tag {
                                    if !thinking_trim.is_empty()
                                        && thinking_trim != last_thinking_sent
                                    {
                                        let thinking_to_send = thinking_trim.clone();
                                        last_thinking_sent = thinking_to_send.clone();
                                        if thinking_stream_id.is_none() {
                                            thinking_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Thinking(
                                                ThinkingPayload {
                                                    task_id: context.task_id.clone(),
                                                    iteration,
                                                    thought: thinking_to_send.clone(),
                                                    stream_id: thinking_stream_id.clone().unwrap(),
                                                    stream_done: false,
                                                    timestamp: Utc::now(),
                                                },
                                            ))
                                            .await?;
                                        {
                                            let runtime_handle = context.react_runtime();
                                            let mut runtime = runtime_handle.write().await;
                                            runtime.record_thought(
                                                react_iteration_index,
                                                stream_text.clone(),
                                                thinking_to_send,
                                            );
                                        }
                                        {
                                            let state_handle = context.state_manager();
                                            let mut state = state_handle.write().await;
                                            state.reset_idle_rounds();
                                        }
                                    }
                                    if can_send_visible
                                        && visible != last_visible_sent
                                        && !last_thinking_sent.trim().is_empty()
                                    {
                                        let visible_to_send = visible.clone();
                                        last_visible_sent = visible_to_send.clone();
                                        if text_stream_id.is_none() {
                                            text_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Text(TextPayload {
                                                task_id: context.task_id.clone(),
                                                iteration,
                                                text: visible_to_send,
                                                stream_id: text_stream_id.clone().unwrap(),
                                                stream_done: false,
                                                timestamp: Utc::now(),
                                            }))
                                            .await?;
                                        {
                                            let state_handle = context.state_manager();
                                            let mut state = state_handle.write().await;
                                            state.reset_idle_rounds();
                                        }
                                    }
                                } else if can_send_visible && visible != last_visible_sent {
                                    let visible_to_send = visible.clone();
                                    last_visible_sent = visible_to_send.clone();
                                    if text_stream_id.is_none() {
                                        text_stream_id = Some(Uuid::new_v4().to_string());
                                    }
                                    context
                                        .send_progress(TaskProgressPayload::Text(TextPayload {
                                            task_id: context.task_id.clone(),
                                            iteration,
                                            text: visible_to_send,
                                            stream_id: text_stream_id.clone().unwrap(),
                                            stream_done: false,
                                            timestamp: Utc::now(),
                                        }))
                                        .await?;
                                    {
                                        let state_handle = context.state_manager();
                                        let mut state = state_handle.write().await;
                                        state.reset_idle_rounds();
                                    }
                                }
                            }

                            if let Some(calls) = tool_calls {
                                for call in calls {
                                    // 去重后立刻通告工具调用（EKO 风格的 tool_use）
                                    if announced_tool_ids.insert(call.id.clone()) {
                                        context
                                            .send_progress(TaskProgressPayload::ToolUse(
                                                ToolUsePayload {
                                                    task_id: context.task_id.clone(),
                                                    iteration,
                                                    tool_id: call.id.clone(),
                                                    tool_name: call.name.clone(),
                                                    params: call.arguments.clone(),
                                                    timestamp: Utc::now(),
                                                },
                                            ))
                                            .await?;
                                    }
                                    {
                                        let runtime_handle = context.react_runtime();
                                        let mut runtime = runtime_handle.write().await;
                                        runtime.record_action(
                                            react_iteration_index,
                                            call.name.clone(),
                                            call.arguments.clone(),
                                        );
                                    }
                                    {
                                        let chain_handle = context.chain();
                                        let mut chain = chain_handle.write().await;
                                        let mut entry =
                                            ToolChain::new(&call, &llm_request_snapshot);
                                        entry.update_params(call.arguments.clone());
                                        chain.push(entry);
                                    }
                                    {
                                        let state_handle = context.state_manager();
                                        let mut state = state_handle.write().await;
                                        state.reset_idle_rounds();
                                    }
                                    pending_tool_calls.push(call);
                                }
                            }
                        }
                        crate::llm::types::LLMStreamChunk::Finish {
                            finish_reason,
                            usage,
                        } => {
                            if let Some(reason_enum) = map_finish_reason(&finish_reason) {
                                finish_reason_enum = Some(reason_enum);
                            }
                            if finish_reason == "tool_calls" || !pending_tool_calls.is_empty() {
                                finished_with_tool_calls = true;
                            }
                            // 关闭进行中的 thinking / text 流
                            if let Some(tsid) = thinking_stream_id.clone() {
                                if !last_thinking_sent.trim().is_empty() {
                                    context
                                        .send_progress(TaskProgressPayload::Thinking(
                                            ThinkingPayload {
                                                task_id: context.task_id.clone(),
                                                iteration,
                                                thought: last_thinking_sent.clone(),
                                                stream_id: tsid,
                                                stream_done: true,
                                                timestamp: Utc::now(),
                                            },
                                        ))
                                        .await?;
                                }
                            }
                            if let Some(xsid) = text_stream_id.clone() {
                                if !last_visible_sent.trim().is_empty() {
                                    context
                                        .send_progress(TaskProgressPayload::Text(TextPayload {
                                            task_id: context.task_id.clone(),
                                            iteration,
                                            text: last_visible_sent.clone(),
                                            stream_id: xsid,
                                            stream_done: true,
                                            timestamp: Utc::now(),
                                        }))
                                        .await?;
                                }
                            }
                            // 发送 Finish 事件
                            context
                                .send_progress(TaskProgressPayload::Finish(FinishPayload {
                                    task_id: context.task_id.clone(),
                                    iteration,
                                    finish_reason,
                                    usage,
                                    timestamp: Utc::now(),
                                }))
                                .await?;
                            break;
                        }
                        crate::llm::types::LLMStreamChunk::Error { error } => {
                            return Err(TaskExecutorError::InternalError(format!(
                                "LLM流式错误: {}",
                                error
                            ))
                            .into());
                        }
                    },
                    Err(e) => {
                        return Err(TaskExecutorError::InternalError(format!(
                            "LLM流式管道错误: {}",
                            e
                        ))
                        .into());
                    }
                }
            }

            let (final_thinking_text, final_visible_text, _) =
                split_thinking_sections(&stream_text);
            let final_thinking_trimmed = final_thinking_text.trim().to_string();
            let final_visible_trimmed = final_visible_text.trim().to_string();

            if finished_with_tool_calls {
                // 执行工具调用并继续下一轮迭代
                for tool_call in pending_tool_calls.clone() {
                    let result = self
                        .execute_tool_call(&context, iteration, tool_call.clone())
                        .await?;

                    let outcome = tool_call_result_to_outcome(&result);

                    {
                        let chain_handle = context.chain();
                        let mut chain = chain_handle.write().await;
                        chain.update_tool_result(&result.call_id, outcome.clone());
                    }

                    {
                        let runtime_handle = context.react_runtime();
                        let mut runtime = runtime_handle.write().await;
                        runtime.record_observation(
                            react_iteration_index,
                            result.tool_name.clone(),
                            outcome.clone(),
                        );
                        if result.is_error {
                            runtime.fail_iteration(
                                react_iteration_index,
                                format!("Tool {} failed", result.tool_name),
                            );
                        } else {
                            runtime.reset_error_counter();
                            runtime.reset_idle_rounds();
                        }
                    }
                }

                context
                    .add_llm_response(LLMResponseParsed {
                        thinking: None,
                        tool_calls: Some(pending_tool_calls),
                        final_answer: None,
                        raw_content: String::new(),
                    })
                    .await;
                context.save_context_snapshot().await?;
                continue;
            }
            // 没有工具调用时，本轮对话已完成（Text 流和 Finish 已发送）

            if final_visible_trimmed.is_empty() {
                {
                    let runtime_handle = context.react_runtime();
                    let mut runtime = runtime_handle.write().await;
                    runtime.mark_idle_round();
                }
                {
                    let state_handle = context.state_manager();
                    let mut state = state_handle.write().await;
                    state.mark_idle_round();
                }
                continue;
            }

            {
                let runtime_handle = context.react_runtime();
                let mut runtime = runtime_handle.write().await;
                runtime.complete_iteration(
                    react_iteration_index,
                    if final_visible_trimmed.is_empty() {
                        None
                    } else {
                        Some(final_visible_trimmed.clone())
                    },
                    finish_reason_enum.clone(),
                );
            }

            context
                .add_llm_response(LLMResponseParsed {
                    thinking: if final_thinking_trimmed.is_empty() {
                        None
                    } else {
                        Some(final_thinking_trimmed.clone())
                    },
                    tool_calls: None,
                    final_answer: if final_visible_trimmed.is_empty() {
                        None
                    } else {
                        Some(final_visible_trimmed.clone())
                    },
                    raw_content: stream_text.clone(),
                })
                .await;
            context.save_context_snapshot().await?;
            break;
        }

        info!("ReAct loop completed for task: {}", context.task_id);
        Ok(())
    }

    /// 创建任务上下文
    pub(crate) async fn create_task_context(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<TaskContext> {
        // 应用配置覆盖
        let mut config = self.default_config.clone();
        if let Some(overrides) = params.config_overrides {
            self.apply_config_overrides(&mut config, overrides)?;
        }

        // 创建新任务
        let mut task = AgentTask::new(params.conversation_id, params.user_prompt)
            .with_max_iterations(config.max_iterations);

        if let Some(config_json) = serde_json::to_string(&config).ok() {
            task = task.with_config(serde_json::from_str(&config_json)?);
        }

        // 保存到数据库
        let task_id = self.repositories.agent_tasks().create(&task).await?;
        task.task_id = task_id.clone();

        // 发送任务创建事件
        if let Some(channel) = &progress_channel {
            channel.send(TaskProgressPayload::TaskCreated(TaskCreatedPayload {
                task_id: task_id.clone(),
                conversation_id: task.conversation_id,
                user_prompt: task.user_prompt.clone(),
            }))?;
        }

        // 创建任务上下文
        let context =
            TaskContext::new(task, config, progress_channel, self.repositories.clone()).await?;

        // 使用提示词构建器生成组件化的 system 与 user 提示词，并覆盖默认初始消息
        // 1) 收集工具信息并转换为简化的 ToolSchema 列表
        let tool_schemas_full = self.tool_registry.get_tool_schemas().await;
        let simple_tool_schemas: Vec<ToolSchema> = tool_schemas_full
            .into_iter()
            .map(|s| ToolSchema {
                name: s.name,
                description: s.description,
                parameters: s.parameters,
            })
            .collect();

        // 2) 构造 Agent / Task / Context（用于 Prompt 组件）
        let tool_names: Vec<String> = simple_tool_schemas.iter().map(|t| t.name.clone()).collect();
        let agent_info = Agent {
            name: "OrbitX Agent".to_string(),
            description: "An AI coding assistant for OrbitX".to_string(),
            capabilities: vec![],
            tools: tool_names,
        };

        // 将存储层 AgentTask 映射为提示词模块的 Task 结构
        let task_for_prompt = Task {
            id: context.task_id.clone(),
            conversation_id: context.conversation_id,
            user_prompt: context.user_prompt.clone(),
            xml: None,
            status: crate::agent::types::TaskStatus::Created,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 3) 构建 system / user 提示词
        let mut prompt_ctx = AgentContext::default();
        prompt_ctx.additional_context.insert(
            "taskPrompt".to_string(),
            serde_json::Value::String(context.user_prompt.clone()),
        );
        let prompt_ctx = Some(prompt_ctx);

        let system_prompt = build_agent_system_prompt(
            agent_info.clone(),
            Some(task_for_prompt.clone()),
            prompt_ctx.clone(),
            simple_tool_schemas.clone(),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("构建系统提示词失败: {}", e))?;

        let user_prompt = build_agent_user_prompt(
            agent_info,
            Some(task_for_prompt),
            prompt_ctx,
            simple_tool_schemas.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("构建用户提示词失败: {}", e))?;

        // 4) 覆盖初始化的两条消息
        context
            .set_initial_prompts(system_prompt, user_prompt)
            .await?;

        Ok(context)
    }

    /// 恢复任务上下文
    async fn restore_task_context(
        &self,
        task_id: String,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<TaskContext> {
        TaskContext::restore(task_id, progress_channel, self.repositories.clone()).await
    }

    /// 处理任务错误
    pub(crate) async fn handle_task_error(
        &self,
        task_id: &str,
        error: anyhow::Error,
        context: Arc<TaskContext>,
    ) {
        error!("Task {} error: {}", task_id, error);

        // 更新任务状态为错误
        if let Err(e) = context.set_status(AgentTaskStatus::Error).await {
            error!("Failed to update task status to error: {}", e);
        }

        // 发送错误事件
        if let Err(e) = context
            .send_progress(TaskProgressPayload::TaskError(TaskErrorPayload {
                task_id: task_id.to_string(),
                iteration: context.current_iteration().await,
                error_message: error.to_string(),
                error_type: "TaskExecutorError".to_string(),
                is_recoverable: false,
                timestamp: Utc::now(),
            }))
            .await
        {
            error!("Failed to send error event: {}", e);
        }

        // 记录错误到执行日志
        if let Err(e) = self
            .log_execution_step(
                &context,
                context.current_iteration().await,
                ExecutionStepType::Error,
                serde_json::json!({
                    "error": error.to_string(),
                    "error_type": "TaskExecutorError",
                    "is_recoverable": false
                }),
            )
            .await
        {
            error!("Failed to log error to execution log: {}", e);
        }
    }

    /// 应用配置覆盖
    fn apply_config_overrides(
        &self,
        config: &mut TaskExecutionConfig,
        overrides: serde_json::Value,
    ) -> TaskExecutorResult<()> {
        if let Some(max_iterations) = overrides.get("max_iterations").and_then(|v| v.as_u64()) {
            config.max_iterations = max_iterations as u32;
        }
        if let Some(max_errors) = overrides.get("max_errors").and_then(|v| v.as_u64()) {
            config.max_errors = max_errors as u32;
        }
        if let Some(verbose) = overrides.get("verbose_logging").and_then(|v| v.as_bool()) {
            config.verbose_logging = verbose;
        }
        Ok(())
    }

    /// 记录执行步骤到日志
    pub(crate) async fn log_execution_step(
        &self,
        context: &TaskContext,
        iteration: u32,
        step_type: ExecutionStepType,
        content: serde_json::Value,
    ) -> TaskExecutorResult<()> {
        let log = AgentExecutionLog::new(context.task_id.clone(), iteration, step_type, content);

        self.repositories.agent_execution_log().create(&log).await?;
        Ok(())
    }
}

fn map_finish_reason(value: &str) -> Option<FinishReason> {
    match value {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" => Some(FinishReason::ToolCalls),
        "content_filter" => Some(FinishReason::ContentFilter),
        _ => None,
    }
}

fn tool_call_result_to_outcome(result: &ToolCallResult) -> ToolOutcome {
    let content = if result.is_error {
        let message = result
            .result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Tool execution failed")
            .to_string();
        ToolResultContent::Error {
            message,
            details: None,
        }
    } else {
        ToolResultContent::Json {
            data: result.result.clone(),
        }
    };

    ToolOutcome {
        content: vec![content],
        is_error: result.is_error,
        execution_time_ms: Some(result.execution_time_ms),
        metadata: None,
    }
}

// 为了支持克隆，我们需要实现Clone trait
impl Clone for TaskExecutor {
    fn clone(&self) -> Self {
        Self {
            repositories: Arc::clone(&self.repositories),
            llm_registry: Arc::clone(&self.llm_registry),
            tool_registry: Arc::clone(&self.tool_registry),
            tool_logger: Arc::clone(&self.tool_logger),
            active_tasks: Arc::clone(&self.active_tasks),
            default_config: self.default_config.clone(),
        }
    }
}
