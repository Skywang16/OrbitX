/*!
 * ReAct Orchestrator - 从 executor.rs 提取的 ReAct 循环核心逻辑
 *
 * 职责：
 * - 管理 ReAct 迭代循环
 * - 处理 LLM 流式响应
 * - 协调工具执行
 * - 管理迭代快照和压缩
 */

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::agent::config::CompactionConfig;
use crate::agent::context::ConversationSummarizer;
use crate::agent::core::context::TaskContext;
use crate::agent::core::iteration_outcome::IterationOutcome;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::{TaskProgressPayload, TextPayload, ThinkingPayload};
use crate::agent::memory::compactor::{CompactionResult, MessageCompactor};
use crate::agent::persistence::AgentPersistence;
use crate::agent::state::iteration::IterationSnapshot;
use crate::agent::state::session::CompressedMemory;
use crate::llm::anthropic_types::{
    ContentBlock, ContentBlockStart, ContentDelta, StreamEvent, SystemPrompt,
};
use crate::storage::DatabaseManager;

/// 内容块累积器（用于流式组装）
enum BlockAccumulator {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input_json: String,
    },
    Thinking(String),
}

/// ReAct 循环编排器
pub struct ReactOrchestrator {
    database: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
}

impl ReactOrchestrator {
    pub fn new(database: Arc<DatabaseManager>, agent_persistence: Arc<AgentPersistence>) -> Self {
        Self {
            database,
            agent_persistence,
        }
    }

    /// ReAct循环执行（核心逻辑）
    ///
    /// 零成本抽象：使用泛型参数H而不是闭包
    /// 编译器会为每个H类型生成特化代码，完全内联
    pub async fn run_react_loop<H>(
        &self,
        context: &TaskContext,
        model_id: &str,
        handler: &H,
    ) -> TaskExecutorResult<()>
    where
        H: crate::agent::core::executor::ReactHandler,
    {
        info!("Starting ReAct loop for task: {}", context.task_id);
        info!(
            "[MODEL_SELECTION] run_react_loop using model_id: {}",
            model_id
        );

        let mut iteration_snapshots: Vec<IterationSnapshot> = Vec::new();

        while !context.should_stop().await {
            context.check_aborted(false).await?;

            // ===== Phase 1: 迭代初始化 =====
            let iteration = context.increment_iteration().await?;
            debug!("Task {} starting iteration {}", context.task_id, iteration);

            let react_iteration_index = {
                let mut state = context.state.write().await;
                state.react_runtime.start_iteration()
            };

            let iter_ctx = context.begin_iteration(iteration).await;

            // ===== Phase 2: 准备消息上下文（零转换） =====
            info!("[MODEL_SELECTION] Using model_id: {}", model_id);

            let tool_registry = context.tool_registry();

            // 性能优化：使用批量读取，一次锁获取所有数据
            let (mut working_messages, mut system_prompt) = context
                .batch_read_state(|state| {
                    (
                        state.execution.messages.iter().cloned().collect::<Vec<_>>(),
                        state.execution.system_prompt.clone(),
                    )
                })
                .await;

            // 摘要（如果需要）
            let summarizer = ConversationSummarizer::new(
                context.conversation_id,
                Arc::clone(&self.agent_persistence),
                Arc::clone(&self.database),
            );
            if let Ok(Some(summary)) = summarizer
                .summarize_if_needed(&model_id, &working_messages, &system_prompt)
                .await
            {
                let sys_text = match &system_prompt {
                    Some(SystemPrompt::Text(t)) => {
                        let capacity = t.len() + summary.summary.len() + 15;
                        let mut buf = String::with_capacity(capacity);
                        buf.push_str(t);
                        buf.push_str("\n\n[summary]\n");
                        buf.push_str(&summary.summary);
                        buf
                    }
                    Some(SystemPrompt::Blocks(_)) | None => summary.summary,
                };
                system_prompt = Some(SystemPrompt::Text(sys_text.clone()));
                let _ = context.update_system_prompt(sys_text).await;
            }

            let compressed_history = context.session().get_compressed_history_text().await;
            if !compressed_history.is_empty() {
                let sys_text = match &system_prompt {
                    Some(SystemPrompt::Text(t)) => {
                        let capacity = t.len() + compressed_history.len() + 15;
                        let mut buf = String::with_capacity(capacity);
                        buf.push_str(t);
                        buf.push_str("\n\n[history]\n");
                        buf.push_str(&compressed_history);
                        buf
                    }
                    Some(SystemPrompt::Blocks(_)) | None => compressed_history,
                };
                system_prompt = Some(SystemPrompt::Text(sys_text.clone()));
                let _ = context.update_system_prompt(sys_text).await;
            }

            // 文件上下文（如有），追加为 user 临时消息
            let recent_iterations = context
                .batch_read_state(|state| state.react_runtime.get_snapshot().iterations.clone())
                .await;
            let builder = handler.get_context_builder(context).await;
            if let Some(file_msg) = builder.build_file_context_message(&recent_iterations).await {
                working_messages.push(file_msg);
            }

            // 消息压缩（超过上下文窗口时）
            let context_window = self
                .get_model_context_window(&model_id)
                .await
                .unwrap_or(128_000);
            let compaction_result = MessageCompactor::new()
                .with_config(CompactionConfig::default())
                .compact_if_needed(
                    working_messages,
                    system_prompt.clone(),
                    &model_id,
                    context_window,
                )
                .await
                .map_err(|e| {
                    TaskExecutorError::InternalError(format!("Compaction failed: {}", e))
                })?;
            if let CompactionResult::Compacted {
                tokens_saved,
                messages_summarized,
                ..
            } = &compaction_result
            {
                info!(
                    "Compacted {} messages, saved {} tokens",
                    messages_summarized, tokens_saved
                );
            }
            let final_messages = compaction_result.messages();

            let llm_request = handler
                .build_llm_request(
                    context,
                    model_id,
                    &tool_registry,
                    &context.cwd,
                    Some(final_messages),
                )
                .await?;

            // 打印 system prompt
            if let Some(ref sp) = llm_request.system {
                if let SystemPrompt::Text(text) = sp {
                    println!(
                        "\n{}\nFINAL SYSTEM PROMPT:\n{}\n{}\n{}\n",
                        "=".repeat(80),
                        "=".repeat(80),
                        text,
                        "=".repeat(80)
                    );
                }
            }

            let llm_service = crate::llm::service::LLMService::new(Arc::clone(&self.database));
            let cancel_token = context.register_step_token();
            let mut stream = llm_service
                .call_stream(llm_request, cancel_token)
                .await
                .map_err(|e| {
                    TaskExecutorError::InternalError(format!("LLM stream call failed: {}", e))
                })?;

            // 新的流处理状态
            let mut current_blocks: HashMap<usize, BlockAccumulator> = HashMap::new();
            let mut text_content: Vec<String> = Vec::new();
            let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();
            let mut pending_tool_calls: Vec<(String, String, Value)> = Vec::new();

            let mut thinking_stream_id: Option<String> = None;
            let mut text_stream_id: Option<String> = None;

            // ===== Phase 3: 处理 Anthropic StreamEvent =====
            while let Some(item) = stream.next().await {
                context.check_aborted(true).await?;

                match item {
                    Ok(StreamEvent::MessageStart { message }) => {
                        tracing::debug!("Message started: {}", message.id);
                    }
                    Ok(StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    }) => match content_block {
                        ContentBlockStart::Text { text } => {
                            current_blocks.insert(index, BlockAccumulator::Text(text));
                        }
                        ContentBlockStart::ToolUse { id, name } => {
                            current_blocks.insert(
                                index,
                                BlockAccumulator::ToolUse {
                                    id,
                                    name,
                                    input_json: String::new(),
                                },
                            );
                        }
                        ContentBlockStart::Thinking { thinking } => {
                            current_blocks.insert(index, BlockAccumulator::Thinking(thinking));
                        }
                    },
                    Ok(StreamEvent::ContentBlockDelta { index, delta }) => {
                        if let Some(block) = current_blocks.get_mut(&index) {
                            match delta {
                                ContentDelta::TextDelta { text } => {
                                    if let BlockAccumulator::Text(s) = block {
                                        s.push_str(&text);
                                        if text_stream_id.is_none() {
                                            text_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Text(TextPayload {
                                                task_id: context.task_id.to_string(),
                                                iteration,
                                                text,
                                                stream_id: text_stream_id.clone().unwrap(),
                                                stream_done: false,
                                                timestamp: Utc::now(),
                                            }))
                                            .await?;
                                        iter_ctx.append_output(&s).await;
                                    }
                                }
                                ContentDelta::InputJsonDelta { partial_json } => {
                                    if let BlockAccumulator::ToolUse { input_json, .. } = block {
                                        input_json.push_str(&partial_json);
                                    }
                                }
                                ContentDelta::ThinkingDelta { thinking } => {
                                    if let BlockAccumulator::Thinking(s) = block {
                                        s.push_str(&thinking);
                                        if thinking_stream_id.is_none() {
                                            thinking_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Thinking(
                                                ThinkingPayload {
                                                    task_id: context.task_id.to_string(),
                                                    iteration,
                                                    thought: thinking,
                                                    stream_id: thinking_stream_id.clone().unwrap(),
                                                    stream_done: false,
                                                    timestamp: Utc::now(),
                                                },
                                            ))
                                            .await?;
                                        iter_ctx.append_thinking(&s).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockStop { index }) => {
                        if let Some(block) = current_blocks.remove(&index) {
                            match block {
                                BlockAccumulator::Text(text) => {
                                    if !text.is_empty() {
                                        text_content.push(text);
                                    }
                                }
                                BlockAccumulator::ToolUse {
                                    id,
                                    name,
                                    input_json,
                                } => {
                                    let input: Value = serde_json::from_str(&input_json).unwrap_or(
                                        serde_json::json!({"_streaming_args": input_json}),
                                    );
                                    tool_use_blocks.push(ContentBlock::ToolUse {
                                        id: id.clone(),
                                        name: name.clone(),
                                        input: input.clone(),
                                    });

                                    context.state.write().await.react_runtime.record_action(
                                        react_iteration_index,
                                        name.clone(),
                                        input.clone(),
                                    );
                                    iter_ctx
                                        .add_tool_call(id.clone(), name.clone(), input.clone())
                                        .await;
                                    pending_tool_calls.push((id, name, input));
                                }
                                BlockAccumulator::Thinking(_) => {}
                            }
                        }
                    }
                    Ok(StreamEvent::MessageDelta { delta, usage }) => {
                        let _ = (delta, usage);
                    }
                    Ok(StreamEvent::MessageStop) => {
                        break;
                    }
                    Ok(StreamEvent::Ping) => {}
                    Ok(StreamEvent::Error { error }) => {
                        return Err(TaskExecutorError::InternalError(error.message));
                    }
                    Err(e) => {
                        return Err(TaskExecutorError::InternalError(e.to_string()));
                    }
                }
            }

            // ===== Phase 4: 将累积内容写入上下文 =====
            let final_text = if !text_content.is_empty() {
                Some(text_content.join("\n"))
            } else {
                None
            };
            context
                .add_assistant_message(final_text.clone(), Some(tool_use_blocks))
                .await;

            // ===== Phase 5: 分类迭代结果 =====
            let outcome = if !pending_tool_calls.is_empty() {
                IterationOutcome::ContinueWithTools {
                    tool_calls: pending_tool_calls.clone(),
                }
            } else if final_text
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                IterationOutcome::Complete {
                    thinking: None,
                    output: final_text.clone(),
                }
            } else {
                IterationOutcome::Empty
            };

            debug!("Iteration {} outcome: {}", iteration, outcome.description());

            // ===== Phase 6: 根据结果执行动作 =====
            match outcome {
                IterationOutcome::ContinueWithTools { ref tool_calls } => {
                    info!(
                        "Iteration {}: executing {} tools",
                        iteration,
                        tool_calls.len()
                    );

                    let deduplicated_calls =
                        crate::agent::core::utils::deduplicate_tool_uses(tool_calls);
                    if deduplicated_calls.len() < tool_calls.len() {
                        let duplicates_count = tool_calls.len() - deduplicated_calls.len();
                        warn!(
                            "Detected {} duplicate tool calls in iteration {}",
                            duplicates_count, iteration
                        );

                        let _ = context
                            .set_system_prompt(format!(
                                "<system-reminder type=\"duplicate-tools\">\n\
                                 You called {} duplicate tool(s) in this iteration.\n\
                                 The results haven't changed. Please use the existing results instead of re-calling the same tools.\n\
                                 </system-reminder>",
                                duplicates_count
                            ))
                            .await;
                    }

                    let results = handler
                        .execute_tools(context, iteration, deduplicated_calls)
                        .await?;

                    for result in results {
                        iter_ctx.add_tool_result(result.clone()).await;

                        let outcome =
                            crate::agent::core::utils::tool_call_result_to_outcome(&result);
                        context
                            .with_chain_mut({
                                let call_id = result.call_id.clone();
                                let outcome_for_chain = outcome.clone();
                                move |chain| {
                                    chain.update_tool_result(&call_id, outcome_for_chain);
                                }
                            })
                            .await;

                        {
                            let mut state = context.state.write().await;
                            state.react_runtime.record_observation(
                                react_iteration_index,
                                result.tool_name.clone(),
                                outcome,
                            );

                            if result.is_error {
                                state.react_runtime.fail_iteration(
                                    react_iteration_index,
                                    format!("Tool {} failed", result.tool_name),
                                );
                            } else {
                                state.react_runtime.reset_error_counter();
                            }
                        }
                    }

                    if let Some(loop_warning) =
                        crate::agent::react::LoopDetector::detect_loop_pattern(context, iteration)
                            .await
                    {
                        warn!("Loop pattern detected in iteration {}", iteration);
                        let _ = context.set_system_prompt(loop_warning).await;
                    }

                    Self::finalize_iteration(context, &mut iteration_snapshots).await?;
                    continue;
                }

                IterationOutcome::Complete { thinking, output } => {
                    info!(
                        "Iteration {}: task complete - {}",
                        iteration,
                        match (&thinking, &output) {
                            (Some(_), Some(_)) => "thinking + output",
                            (Some(_), None) => "thinking only",
                            (None, Some(_)) => "output only",
                            (None, None) => "empty (unexpected)",
                        }
                    );

                    context
                        .state
                        .write()
                        .await
                        .react_runtime
                        .complete_iteration(react_iteration_index, output.clone(), None);

                    Self::finalize_iteration(context, &mut iteration_snapshots).await?;
                    break;
                }

                IterationOutcome::Empty => {
                    warn!(
                        "Iteration {}: empty response - terminating immediately",
                        iteration
                    );

                    Self::finalize_iteration(context, &mut iteration_snapshots).await?;
                    break;
                }
            }
        }

        info!("ReAct loop completed for task: {}", context.task_id);
        if !iteration_snapshots.is_empty() {
            Self::compress_iteration_batch(context, &iteration_snapshots).await?;
        }
        Ok(())
    }

    async fn finalize_iteration(
        context: &TaskContext,
        snapshots: &mut Vec<IterationSnapshot>,
    ) -> TaskExecutorResult<()> {
        if let Some(snapshot) = context.end_iteration().await {
            let tool_calls = snapshot.tools_used.len() as u32;
            let files = snapshot.files_touched.len() as u32;
            context
                .session()
                .update_stats(|stats| {
                    stats.total_iterations = stats.total_iterations.saturating_add(1);
                    stats.total_tool_calls = stats.total_tool_calls.saturating_add(tool_calls);
                    stats.files_read = stats.files_read.saturating_add(files);
                })
                .await;
            snapshots.push(snapshot);
            if snapshots.len() >= 5 {
                Self::compress_iteration_batch(context, snapshots).await?;
                snapshots.clear();
            }
        }
        Ok(())
    }

    async fn compress_iteration_batch(
        context: &TaskContext,
        snapshots: &[IterationSnapshot],
    ) -> TaskExecutorResult<()> {
        if snapshots.is_empty() {
            return Ok(());
        }

        let start_iter = snapshots.first().unwrap().iteration;
        let end_iter = snapshots.last().unwrap().iteration;

        let mut files = Vec::new();
        let mut tools = Vec::new();
        let mut summary_parts = Vec::new();

        for snapshot in snapshots {
            files.extend(snapshot.files_touched.clone());
            tools.extend(snapshot.tools_used.clone());
            summary_parts.push(snapshot.summarize());
        }

        files.sort();
        files.dedup();
        tools.sort();
        tools.dedup();

        let memory = CompressedMemory {
            created_at: Utc::now(),
            iteration_range: (start_iter, end_iter),
            summary: summary_parts.join("\n"),
            files_touched: files,
            tools_used: tools,
            tokens_saved: 0,
        };

        context.session().add_compressed_memory(memory).await;

        Ok(())
    }

    async fn get_model_context_window(&self, model_id: &str) -> Option<u32> {
        let model = crate::storage::repositories::AIModels::new(&self.database)
            .find_by_id(model_id)
            .await
            .ok()??;

        if let Some(options) = model.options {
            if let Some(max_tokens) = options.get("maxContextTokens") {
                if let Some(value) = max_tokens.as_u64() {
                    return Some(value as u32);
                }
            }
        }

        None
    }
}
