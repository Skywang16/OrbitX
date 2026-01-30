/*!
 * ReAct Orchestrator - 从 executor.rs 提取的 ReAct 循环核心逻辑
 *
 * 职责：
 * - 管理 ReAct 迭代循环
 * - 处理 LLM 流式响应
 * - 协调工具执行
 * - 管理迭代快照和压缩
 */

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

use crate::agent::compaction::{
    CompactionConfig, CompactionService, CompactionTrigger, SessionMessageLoader,
};
use crate::agent::core::context::TaskContext;
use crate::agent::core::iteration_outcome::IterationOutcome;
use crate::agent::core::utils::should_render_tool_block;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::persistence::AgentPersistence;
use crate::agent::prompt::PromptBuilder;
use crate::agent::state::iteration::{IterationContext, IterationSnapshot};
use crate::agent::types::{Block, TextBlock, ThinkingBlock, ToolBlock, ToolStatus};
use crate::agent::terminal::AgentTerminalManager;
use crate::agent::tools::ToolDescriptionContext;
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
        last_ui_update: Instant,
        last_ui_len: usize,
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
        tracing::info!("🔄 Starting ReAct loop with model: {}", model_id);
        let mut fabricated_tool_output_count: u32 = 0;

        while !context.should_stop().await {
            context.check_aborted_async(false).await?;

            // ===== Phase 1: 迭代初始化 =====
            let iteration = context.increment_iteration().await?;
            // Clear transient system reminders (e.g. loop warnings) each iteration; they are
            // meant to influence the *next* step only, not permanently replace the base prompt.
            context.set_system_prompt_overlay(None).await?;
            if let Some(manager) = AgentTerminalManager::global() {
                if let Some(overlay) = manager.build_prompt_overlay(context.session_id) {
                    let _ = context
                        .set_system_prompt_overlay(Some(SystemPrompt::Text(overlay)))
                        .await;
                }
            }

            let react_iteration_index = {
                let mut react = context.states.react_runtime.write().await;
                react.start_iteration()
            };

            let mut iter_ctx = IterationContext::new(iteration, context.session());

            // ===== Phase 2: 准备消息上下文（从 messages 表加载，Summary 作为断点） =====

            let tool_registry = context.tool_registry();
            let tool_names: HashSet<String> = tool_registry
                .get_tool_schemas_with_context(&ToolDescriptionContext {
                    cwd: context.cwd.to_string(),
                })
                .into_iter()
                .map(|schema| schema.name.to_lowercase())
                .collect();

            // 文件上下文（如有），追加为 user 临时消息
            let recent_iterations = {
                let react = context.states.react_runtime.read().await;
                react.get_snapshot().iterations.clone()
            };
            let builder = handler.get_context_builder(context).await;
            let context_window =
                crate::agent::utils::get_model_context_window(&self.database, model_id)
                    .await
                    .ok_or_else(|| {
                        TaskExecutorError::ConfigurationError(
                            "Missing model option `maxContextTokens` for compaction".to_string(),
                        )
                    })?;
            self.maybe_compact_session(context, model_id, context_window)
                .await?;

            let loader = SessionMessageLoader::new(Arc::clone(&self.agent_persistence));
            let mut final_messages = loader
                .load_for_llm(context.session_id)
                .await
                .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

            if let Some(file_msg) = builder.build_file_context_message(&recent_iterations).await {
                final_messages.push(file_msg);
            }

            let llm_request = handler
                .build_llm_request(
                    context,
                    model_id,
                    &tool_registry,
                    &context.cwd,
                    Some(final_messages),
                )
                .await?;

            let llm_service = crate::llm::service::LLMService::new(Arc::clone(&self.database));
            let cancel_token = context.create_stream_cancel_token();

            let mut stream = llm_service
                .call_stream(llm_request, cancel_token)
                .await
                .map_err(|e| {
                    TaskExecutorError::InternalError(format!("LLM stream call failed: {e}"))
                })?;

            // 新的流处理状态
            let mut current_blocks: HashMap<usize, BlockAccumulator> = HashMap::new();
            let mut text_content: Vec<String> = Vec::new();
            let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();
            let mut pending_tool_calls: Vec<(String, String, Value)> = Vec::new();
            let mut tool_block_visibility: HashMap<String, bool> = HashMap::new();
            let mut tool_block_started_at: HashMap<String, chrono::DateTime<chrono::Utc>> =
                HashMap::new();

            let mut thinking_stream_id: Option<String> = None;
            let mut text_stream_id: Option<String> = None;
            let mut thinking_created = false;
            let mut text_created = false;

            // ===== Phase 3: 处理 Anthropic StreamEvent =====
            while let Some(item) = stream.next().await {
                if context.is_aborted() {
                    return Err(TaskExecutorError::TaskInterrupted);
                }
                context.check_aborted_async(true).await?;

                match item {
                    Ok(StreamEvent::MessageStart { .. }) => {}
                    Ok(StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    }) => match content_block {
                        ContentBlockStart::Text { text } => {
                            current_blocks.insert(index, BlockAccumulator::Text(text));
                        }
                        ContentBlockStart::ToolUse { id, name } => {
                            let tool_id = id.clone();
                            let tool_name = name.clone();
                            current_blocks.insert(
                                index,
                                BlockAccumulator::ToolUse {
                                    id,
                                    name,
                                    input_json: String::new(),
                                    last_ui_update: Instant::now(),
                                    last_ui_len: 0,
                                },
                            );
                            let should_render = should_render_tool_block(context, &tool_name).await;
                            tool_block_visibility.insert(tool_id.clone(), should_render);
                            if should_render {
                                let now = Utc::now();
                                tool_block_started_at.insert(tool_id.clone(), now);
                                context
                                    .assistant_append_block(Block::Tool(ToolBlock {
                                        id: tool_id.clone(),
                                        call_id: tool_id.clone(),
                                        name: tool_name.clone(),
                                        status: ToolStatus::Pending,
                                        input: Value::Object(serde_json::Map::new()),
                                        output: None,
                                        compacted_at: None,
                                        started_at: now,
                                        finished_at: None,
                                        duration_ms: None,
                                    }))
                                    .await?;
                            }
                        }
                        ContentBlockStart::Thinking { thinking } => {
                            current_blocks.insert(index, BlockAccumulator::Thinking(thinking));
                        }
                    },
                    Ok(StreamEvent::ContentBlockDelta { index, delta }) => {
                        if let Some(block) = current_blocks.get_mut(&index) {
                            match delta {
                                ContentDelta::Text { text } => {
                                    if let BlockAccumulator::Text(s) = block {
                                        s.push_str(&text);
                                        if text_stream_id.is_none() {
                                            text_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        let id = text_stream_id.clone().unwrap();
                                        let block = Block::Text(TextBlock {
                                            id: id.clone(),
                                            content: s.clone(),
                                            is_streaming: true,
                                        });
                                        if text_created {
                                            context.assistant_update_block(&id, block).await?;
                                        } else {
                                            context.assistant_append_block(block).await?;
                                            text_created = true;
                                        }
                                        iter_ctx.append_output(&text);
                                    }
                                }
                                ContentDelta::InputJson { partial_json } => {
                                    if let BlockAccumulator::ToolUse {
                                        id,
                                        name,
                                        input_json,
                                        last_ui_update,
                                        last_ui_len,
                                    } = block
                                    {
                                        input_json.push_str(&partial_json);

                                        if !tool_block_visibility
                                            .get(id)
                                            .copied()
                                            .unwrap_or(false)
                                        {
                                            continue;
                                        }

                                        const MIN_UI_UPDATE_INTERVAL: Duration =
                                            Duration::from_millis(750);
                                        const MIN_BYTES_DELTA_FOR_UPDATE: usize = 2048;

                                        let now = Instant::now();
                                        let bytes = input_json.len();
                                        let bytes_delta = bytes.saturating_sub(*last_ui_len);
                                        if bytes_delta < MIN_BYTES_DELTA_FOR_UPDATE
                                            && now.duration_since(*last_ui_update)
                                                < MIN_UI_UPDATE_INTERVAL
                                        {
                                            continue;
                                        }

                                        *last_ui_update = now;
                                        *last_ui_len = bytes;

                                        let started_at = tool_block_started_at
                                            .get(id)
                                            .cloned()
                                            .unwrap_or_else(Utc::now);
                                        let _ = context
                                            .assistant_upsert_block(Block::Tool(ToolBlock {
                                                id: id.clone(),
                                                call_id: id.clone(),
                                                name: name.clone(),
                                                status: ToolStatus::Pending,
                                                input: serde_json::json!({
                                                    "__streaming": true,
                                                    "__inputBytes": bytes,
                                                }),
                                                output: None,
                                                compacted_at: None,
                                                started_at,
                                                finished_at: None,
                                                duration_ms: None,
                                            }))
                                            .await;
                                    }
                                }
                                ContentDelta::Thinking { thinking } => {
                                    if let BlockAccumulator::Thinking(s) = block {
                                        s.push_str(&thinking);
                                        if thinking_stream_id.is_none() {
                                            thinking_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        let id = thinking_stream_id.clone().unwrap();
                                        let block = Block::Thinking(ThinkingBlock {
                                            id: id.clone(),
                                            content: s.clone(),
                                            is_streaming: true,
                                        });
                                        if thinking_created {
                                            context.assistant_update_block(&id, block).await?;
                                        } else {
                                            context.assistant_append_block(block).await?;
                                            thinking_created = true;
                                        }
                                        iter_ctx.append_thinking(&thinking);
                                    }
                                }
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockStop { index }) => {
                        if let Some(block) = current_blocks.remove(&index) {
                            match block {
                                BlockAccumulator::Text(text) => {
                                    if text_created {
                                        if let Some(id) = &text_stream_id {
                                            let block = Block::Text(TextBlock {
                                                id: id.clone(),
                                                content: text.clone(),
                                                is_streaming: false,
                                            });
                                            let _ = context.assistant_update_block(id, block).await;
                                        }
                                    }
                                    if !text.is_empty() {
                                        text_content.push(text);
                                    }
                                }
                                BlockAccumulator::ToolUse { id, name, input_json, .. } => {
                                    // Some OpenAI-compatible models (e.g., GLM) may send
                                    // duplicate arguments via both `tool_calls` and
                                    // `function_call`, resulting in concatenated JSON like
                                    // `{...}{...}`. We defensively parse only the first
                                    // valid JSON object using a streaming deserializer.
                                    let input: Value = {
                                        let mut de =
                                            serde_json::Deserializer::from_str(&input_json)
                                                .into_iter::<Value>();
                                        de.next()
                                            .ok_or_else(|| {
                                                TaskExecutorError::InternalError(
                                                    "Empty tool input JSON from stream"
                                                        .to_string(),
                                                )
                                            })?
                                            .map_err(|err| {
                                                TaskExecutorError::InternalError(format!(
                                                    "Invalid tool input JSON from stream: {err}"
                                                ))
                                            })?
                                    };
                                    tool_use_blocks.push(ContentBlock::ToolUse {
                                        id: id.clone(),
                                        name: name.clone(),
                                        input: input.clone(),
                                    });

                                    if tool_block_visibility.get(&id).copied().unwrap_or(false) {
                                        let now = Utc::now();
                                        let started_at = tool_block_started_at
                                            .get(&id)
                                            .cloned()
                                            .unwrap_or(now);
                                        context
                                            .assistant_upsert_block(Block::Tool(ToolBlock {
                                                id: id.clone(),
                                                call_id: id.clone(),
                                                name: name.clone(),
                                                status: ToolStatus::Pending,
                                                input: input.clone(),
                                                output: None,
                                                compacted_at: None,
                                                started_at,
                                                finished_at: None,
                                                duration_ms: None,
                                            }))
                                            .await?;
                                    }

                                    context.states.react_runtime.write().await.record_action(
                                        react_iteration_index,
                                        name.clone(),
                                        input.clone(),
                                    );
                                    iter_ctx.add_tool_call(id.clone(), name.clone(), input.clone());
                                    pending_tool_calls.push((id, name, input));
                                }
                                BlockAccumulator::Thinking(thinking) => {
                                    if thinking_created {
                                        if let Some(id) = &thinking_stream_id {
                                            let block = Block::Thinking(ThinkingBlock {
                                                id: id.clone(),
                                                content: thinking,
                                                is_streaming: false,
                                            });
                                            let _ = context.assistant_update_block(id, block).await;
                                        }
                                    }
                                }
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

            if context.is_aborted() {
                return Err(TaskExecutorError::TaskInterrupted);
            }

            // ===== Phase 4: 将累积内容写入上下文 =====
            let final_text = if !text_content.is_empty() {
                Some(text_content.join("\n"))
            } else {
                None
            };

            if pending_tool_calls.is_empty() {
                if let Some(text) = final_text.as_deref() {
                    if contains_fabricated_tool_output(text, &tool_names) {
                        fabricated_tool_output_count =
                            fabricated_tool_output_count.saturating_add(1);

                        if let Some(id) = &text_stream_id {
                            let note = if fabricated_tool_output_count == 1 {
                                "⚠️ Invalid tool output text. Retrying once."
                            } else {
                                "⚠️ Invalid tool output text. Aborting."
                            };
                            let _ = context
                                .assistant_update_block(
                                    id,
                                    Block::Text(TextBlock {
                                        id: id.clone(),
                                        content: note.to_string(),
                                        is_streaming: false,
                                    }),
                                )
                                .await;
                        }

                        if fabricated_tool_output_count == 1 {
                            let reminder = "You must NEVER claim tool results in plain text. \
If a tool is needed, emit a tool call; otherwise reply without tool-result language.";
                            let _ = context
                                .set_system_prompt_overlay(Some(SystemPrompt::Text(format!(
                                    "<system-reminder type=\"invalid-tool-output\">\n{}\n</system-reminder>",
                                    reminder
                                ))))
                                .await;
                            continue;
                        }

                        return Err(TaskExecutorError::InternalError(format!(
                            "LLM output contained fabricated tool results (count={})",
                            fabricated_tool_output_count
                        )));
                    }
                }
            }

            context
                .add_assistant_message(final_text.clone(), Some(tool_use_blocks))
                .await?;

            // ===== Phase 5: 分类迭代结果 =====
            let outcome = if !pending_tool_calls.is_empty() {
                IterationOutcome::ContinueWithTools {
                    tool_calls: pending_tool_calls.clone(),
                }
            } else if matches!(final_text.as_deref(), Some(s) if !s.trim().is_empty()) {
                IterationOutcome::Complete {
                    thinking: None,
                    output: final_text.clone(),
                }
            } else {
                IterationOutcome::Empty
            };

            // ===== Phase 6: 根据结果执行动作 =====
            match outcome {
                IterationOutcome::ContinueWithTools { ref tool_calls } => {
                    let deduplicated_calls =
                        crate::agent::core::utils::deduplicate_tool_uses(tool_calls);
                    if deduplicated_calls.len() < tool_calls.len() {
                        let kept_ids: HashSet<String> = deduplicated_calls
                            .iter()
                            .map(|(id, _, _)| id.clone())
                            .collect();
                        let now = Utc::now();
                        for (call_id, tool_name, input) in tool_calls.iter() {
                            if kept_ids.contains(call_id) {
                                continue;
                            }
                            if tool_block_visibility.get(call_id).copied().unwrap_or(false) {
                                let started_at = tool_block_started_at
                                    .get(call_id)
                                    .cloned()
                                    .unwrap_or(now);
                                context
                                    .assistant_upsert_block(Block::Tool(ToolBlock {
                                        id: call_id.clone(),
                                        call_id: call_id.clone(),
                                        name: tool_name.clone(),
                                        status: ToolStatus::Cancelled,
                                        input: input.clone(),
                                        output: None,
                                        compacted_at: None,
                                        started_at,
                                        finished_at: Some(now),
                                        duration_ms: Some(0),
                                    }))
                                    .await?;
                            }
                        }

                        let duplicates_count = tool_calls.len() - deduplicated_calls.len();
                        warn!(
                            "Detected {} duplicate tool calls in iteration {}",
                            duplicates_count, iteration
                        );

                        let builder = PromptBuilder::new(None);
                        let warning = builder.get_duplicate_tools_warning(duplicates_count);
                        let _ = context
                            .set_system_prompt_overlay(Some(
                                crate::llm::anthropic_types::SystemPrompt::Text(format!(
                                    "<system-reminder type=\"duplicate-tools\">\n{}\n</system-reminder>",
                                    warning
                                )),
                            ))
                            .await;
                    }

                    let results = handler
                        .execute_tools(context, iteration, deduplicated_calls)
                        .await?;

                    for result in results {
                        let outcome =
                            crate::agent::core::utils::tool_call_result_to_outcome(&result);
                        let call_id = result.call_id.clone();
                        let tool_name = result.tool_name.clone();
                        let status = result.status;
                        iter_ctx.add_tool_result(result);

                        context
                            .with_chain_mut({
                                let outcome_for_chain = outcome.clone();
                                move |chain| {
                                    chain.update_tool_result(&call_id, outcome_for_chain);
                                }
                            })
                            .await;

                        {
                            let mut react = context.states.react_runtime.write().await;
                            react.record_observation(
                                react_iteration_index,
                                tool_name.clone(),
                                outcome,
                            );

                            if status != crate::agent::tools::ToolResultStatus::Success {
                                react.fail_iteration(
                                    react_iteration_index,
                                    format!("Tool {tool_name} failed"),
                                );
                            } else {
                                react.reset_error_counter();
                            }
                        }
                    }

                    if let Some(loop_warning) =
                        crate::agent::react::LoopDetector::detect_loop_pattern(context, iteration)
                            .await
                    {
                        warn!("Loop pattern detected in iteration {}", iteration);
                        let _ = context
                            .set_system_prompt_overlay(Some(
                                crate::llm::anthropic_types::SystemPrompt::Text(loop_warning),
                            ))
                            .await;
                    }

                    let snapshot = iter_ctx.finalize();
                    Self::update_session_stats(context, &snapshot).await;
                    continue;
                }

                IterationOutcome::Complete {
                    thinking: _,
                    output,
                } => {
                    tracing::info!("✅ Task completed successfully at iteration {}", iteration);

                    context
                        .states
                        .react_runtime
                        .write()
                        .await
                        .complete_iteration(react_iteration_index, output.clone(), None);

                    let snapshot = iter_ctx.finalize();
                    Self::update_session_stats(context, &snapshot).await;
                    break;
                }

                IterationOutcome::Empty => {
                    warn!(
                        "Iteration {}: empty response - terminating immediately",
                        iteration
                    );

                    tracing::warn!("⚠️  Empty LLM response at iteration {}, terminating task", iteration);

                    let snapshot = iter_ctx.finalize();
                    Self::update_session_stats(context, &snapshot).await;
                    break;
                }
            }
        }

        tracing::info!("🏁 ReAct loop finished");
        Ok(())
    }

    async fn update_session_stats(context: &TaskContext, snapshot: &IterationSnapshot) {
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
    }

    async fn maybe_compact_session(
        &self,
        context: &TaskContext,
        model_id: &str,
        context_window: u32,
    ) -> TaskExecutorResult<()> {
        let service = CompactionService::new(
            Arc::clone(&self.database),
            Arc::clone(&self.agent_persistence),
            CompactionConfig::default(),
        );

        let prepared = service
            .prepare_compaction(context.session_id, context_window, CompactionTrigger::Auto)
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

        let Some(job) = prepared.summary_job else {
            return Ok(());
        };

        context
            .emit_event(crate::agent::types::TaskEvent::MessageCreated {
                task_id: context.task_id.to_string(),
                message: job.summary_message.clone(),
            })
            .await?;

        let completed = service
            .complete_summary_job(job, model_id)
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

        let context_usage = context.calculate_context_usage(model_id).await;
        context
            .emit_event(crate::agent::types::TaskEvent::MessageFinished {
                task_id: context.task_id.to_string(),
                message_id: completed.message_id,
                status: completed.status,
                finished_at: completed.finished_at,
                duration_ms: completed.duration_ms,
                token_usage: None,
                context_usage,
            })
            .await?;

        Ok(())
    }
}

fn contains_fabricated_tool_output(text: &str, tool_names: &HashSet<String>) -> bool {
    if tool_names.is_empty() {
        return false;
    }

    let lower = text.to_lowercase();
    for name in tool_names {
        let name_lower = name.to_lowercase();
        let name_spaced = name_lower.replace('_', " ");
        if lower.contains("tool ")
            && (lower.contains(&name_lower) || lower.contains(&name_spaced))
            && (lower.contains("completed")
                || lower.contains("successfully")
                || lower.contains("failed")
                || lower.contains("error")
                || lower.contains("完成")
                || lower.contains("成功")
                || lower.contains("失败")
                || lower.contains("错误"))
        {
            return true;
        }
    }
    false
}

// Compaction business rules live in `agent/compaction/*` (not in the orchestrator).
