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

use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

use crate::agent::compaction::{CompactionConfig, CompactionService, CompactionTrigger, SessionMessageLoader};
use crate::agent::core::context::TaskContext;
use crate::agent::core::iteration_outcome::IterationOutcome;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::persistence::AgentPersistence;
use crate::agent::state::iteration::{IterationContext, IterationSnapshot};
use crate::agent::types::{Block, TextBlock, ThinkingBlock};
use crate::llm::anthropic_types::{ContentBlock, ContentBlockStart, ContentDelta, StreamEvent};
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
        while !context.should_stop().await {
            context.check_aborted_async(false).await?;

            // ===== Phase 1: 迭代初始化 =====
            let iteration = context.increment_iteration().await?;

            let react_iteration_index = {
                let mut react = context.states.react_runtime.write().await;
                react.start_iteration()
            };

            let iter_ctx = IterationContext::new(iteration, context.session());

            // ===== Phase 2: 准备消息上下文（从 messages 表加载，Summary 作为断点） =====

            let tool_registry = context.tool_registry();

            // 文件上下文（如有），追加为 user 临时消息
            let recent_iterations = {
                let react = context.states.react_runtime.read().await;
                react.get_snapshot().iterations.clone()
            };
            let builder = handler.get_context_builder(context).await;
            let context_window = crate::agent::utils::get_model_context_window(&self.database, model_id)
                .await
                .unwrap_or(128_000);
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
                    TaskExecutorError::InternalError(format!("LLM stream call failed: {}", e))
                })?;

            // 新的流处理状态
            let mut current_blocks: HashMap<usize, BlockAccumulator> = HashMap::new();
            let mut text_content: Vec<String> = Vec::new();
            let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();
            let mut pending_tool_calls: Vec<(String, String, Value)> = Vec::new();

            let mut thinking_stream_id: Option<String> = None;
            let mut text_stream_id: Option<String> = None;
            let mut thinking_created = false;
            let mut text_created = false;

            // ===== Phase 3: 处理 Anthropic StreamEvent =====
            while let Some(item) = stream.next().await {
                if context.is_aborted() {
                    break;
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
                                        iter_ctx.append_output(&text).await;
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
                                        iter_ctx.append_thinking(&thinking).await;
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

                                    context.states.react_runtime.write().await.record_action(
                                        react_iteration_index,
                                        name.clone(),
                                        input.clone(),
                                    );
                                    iter_ctx
                                        .add_tool_call(id.clone(), name.clone(), input.clone())
                                        .await;
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

            // ===== Phase 4: 将累积内容写入上下文 =====
            let final_text = if !text_content.is_empty() {
                Some(text_content.join("\n"))
            } else {
                None
            };
            context
                .add_assistant_message(final_text.clone(), Some(tool_use_blocks))
                .await?;

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

            // ===== Phase 6: 根据结果执行动作 =====
            match outcome {
                IterationOutcome::ContinueWithTools { ref tool_calls } => {
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
                            let mut react = context.states.react_runtime.write().await;
                            react.record_observation(
                                react_iteration_index,
                                result.tool_name.clone(),
                                outcome,
                            );

                            if result.status != crate::agent::tools::ToolResultStatus::Success {
                                react.fail_iteration(
                                    react_iteration_index,
                                    format!("Tool {} failed", result.tool_name),
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
                        let _ = context.set_system_prompt(loop_warning).await;
                    }

                    let snapshot = iter_ctx.finalize().await;
                    Self::update_session_stats(context, &snapshot).await;
                    continue;
                }

                IterationOutcome::Complete {
                    thinking: _,
                    output,
                } => {
                    context
                        .states
                        .react_runtime
                        .write()
                        .await
                        .complete_iteration(react_iteration_index, output.clone(), None);

                    let snapshot = iter_ctx.finalize().await;
                    Self::update_session_stats(context, &snapshot).await;
                    break;
                }

                IterationOutcome::Empty => {
                    warn!(
                        "Iteration {}: empty response - terminating immediately",
                        iteration
                    );

                    let snapshot = iter_ctx.finalize().await;
                    Self::update_session_stats(context, &snapshot).await;
                    break;
                }
            }
        }
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

// Compaction business rules live in `agent/compaction/*` (not in the orchestrator).
