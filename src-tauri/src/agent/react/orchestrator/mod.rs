/*!
 * ReAct Orchestrator - ReAct loop core logic extracted from executor.rs
 *
 * Responsibilities:
 * - Manage ReAct iteration loop
 * - Handle LLM streaming responses
 * - Coordinate tool execution
 * - Manage iteration snapshots and compaction
 */

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::warn;
use uuid::Uuid;

use crate::agent::agents::{visible_task_profiles, AgentConfigLoader};
use crate::agent::compaction::{
    CompactionConfig, CompactionService, CompactionTrigger, ThreadMessageLoader,
};
use crate::agent::core::context::AgentRunContext;
use crate::agent::core::iteration_outcome::IterationOutcome;
use crate::agent::core::utils::should_render_tool_block;
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::persistence::AgentPersistence;
use crate::agent::prompt::PromptBuilder;
use crate::agent::state::iteration::{IterationContext, IterationSnapshot};
use crate::agent::terminal::AgentTerminalManager;
use crate::agent::tools::ToolDescriptionContext;
use crate::agent::types::{Block, TextBlock, ThinkingBlock, ToolBlock, ToolStatus};
use crate::llm::anthropic_types::{
    ContentBlock, ContentBlockStart, ContentDelta, StreamEvent, SystemPrompt,
};
use crate::storage::DatabaseManager;
use sqlx::Row;

/// Content block accumulator (for streaming assembly)
enum BlockAccumulator {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input_json: String,
        last_ui_update: Instant,
        last_ui_len: usize,
    },
    Thinking {
        thinking: String,
        /// Provider-specific metadata (signature, item_id, encrypted_content)
        metadata: Option<crate::llm::anthropic_types::ReasoningBlockMetadata>,
    },
}

/// ReAct loop orchestrator
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

    /// Execute ReAct loop (core logic)
    ///
    /// The compiler generates specialized code for each H type, fully inlined
    pub async fn run_react_loop<H>(
        &self,
        context: &AgentRunContext,
        model_id: &str,
        handler: &H,
    ) -> AgentRunResult<()>
    where
        H: crate::agent::core::executor::ReactHandler,
    {
        tracing::info!("🔄 Starting ReAct loop with model: {}", model_id);
        let mut fabricated_tool_output_count: u32 = 0;
        let mut empty_response_count: u32 = 0;

        while !context.should_stop().await {
            context.check_aborted_async(false).await?;

            // ===== Phase 1: Iteration initialization =====
            let iteration = context.increment_iteration().await?;
            // Clear transient system reminders (e.g. loop warnings) each iteration; they are
            // meant to influence the *next* step only, not permanently replace the base prompt.
            context.set_system_prompt_overlay(None).await?;
            if let Some(manager) = AgentTerminalManager::global() {
                if let Some(overlay) = manager.build_prompt_overlay(context.thread_id) {
                    if let Err(err) = context
                        .set_system_prompt_overlay(Some(SystemPrompt::Text(overlay)))
                        .await
                    {
                        warn!("Failed to apply terminal prompt overlay: {}", err);
                    }
                }
            }

            let react_iteration_index = {
                let mut react = context.states.react_runtime.write().await;
                react.start_iteration()
            };

            let mut iter_ctx = IterationContext::new(iteration, context.session());

            // ===== Phase 2: Prepare message context (load from messages table, Summary as checkpoint) =====

            let tool_registry = context.tool_registry();
            let tool_names: HashSet<String> = tool_registry
                .get_tool_schemas_with_context(&ToolDescriptionContext {
                    cwd: context.cwd.to_string(),
                    agent_type: Some(context.agent_type.to_string()),
                    allowed_subagent_types: load_allowed_task_profiles(
                        context.cwd.as_ref(),
                        context.agent_type.as_ref(),
                    )
                    .await,
                })
                .into_iter()
                .map(|schema| schema.name.to_lowercase())
                .collect();

            // File context (if any), append as user temporary message
            let recent_iterations = {
                let react = context.states.react_runtime.read().await;
                react.get_snapshot().iterations.clone()
            };
            let builder = handler.get_context_builder(context).await;
            let context_window =
                crate::agent::utils::get_model_context_window(&self.database, model_id)
                    .await
                    .ok_or_else(|| {
                        AgentRunError::ConfigurationError(
                            "Missing model option `maxContextTokens` for compaction".to_string(),
                        )
                    })?;
            self.maybe_compact_session(context, model_id, context_window)
                .await?;

            let loader = ThreadMessageLoader::new(Arc::clone(&self.agent_persistence));
            let mut final_messages = loader
                .load_for_llm(context.thread_id)
                .await
                .map_err(|e| AgentRunError::InternalError(e.to_string()))?;

            if let Some(file_msg) = builder.build_file_context_message(&recent_iterations).await {
                final_messages.push(file_msg);
            }

            // ===== Per-turn: inject fresh active subagents into developer context =====
            // Codex pattern: subagents list belongs in the environment/developer context layer,
            // NOT as a user message in the conversation stream. This ensures:
            // 1. The model treats it as authoritative environment state
            // 2. It won't be lost during compaction
            // 3. It doesn't break user→assistant message alternation
            let subagents_sections =
                build_subagents_developer_context(&self.database, context.thread_id).await;
            context
                .set_per_turn_developer_context(subagents_sections)
                .await?;

            // ===== DEBUG: dump full message context sent to LLM =====
            {
                use crate::llm::anthropic_types::{
                    ContentBlock, MessageContent, ToolResultContent,
                };
                tracing::warn!(
                    "===== [DEBUG] LLM context for iteration {} | total messages: {} =====",
                    iteration,
                    final_messages.len()
                );
                for (i, msg) in final_messages.iter().enumerate() {
                    match &msg.content {
                        MessageContent::Text(t) => {
                            let preview: String = t.chars().take(200).collect();
                            tracing::warn!(
                                "  [MSG {i}] role={:?} | Text({} chars): {}{}",
                                msg.role,
                                t.len(),
                                preview,
                                if t.len() > 200 { "..." } else { "" }
                            );
                        }
                        MessageContent::Blocks(blocks) => {
                            tracing::warn!(
                                "  [MSG {i}] role={:?} | Blocks({})",
                                msg.role,
                                blocks.len()
                            );
                            for (j, block) in blocks.iter().enumerate() {
                                match block {
                                    ContentBlock::Text { text, .. } => {
                                        let preview: String = text.chars().take(150).collect();
                                        tracing::warn!(
                                            "    [BLK {j}] Text({} chars): {}{}",
                                            text.len(),
                                            preview,
                                            if text.len() > 150 { "..." } else { "" }
                                        );
                                    }
                                    ContentBlock::ToolUse { id, name, input } => {
                                        let input_str = input.to_string();
                                        let preview: String = input_str.chars().take(150).collect();
                                        tracing::warn!(
                                            "    [BLK {j}] ToolUse id={} name={} input({} chars): {}{}",
                                            id,
                                            name,
                                            input_str.len(),
                                            preview,
                                            if input_str.len() > 150 { "..." } else { "" }
                                        );
                                    }
                                    ContentBlock::ToolResult {
                                        tool_use_id,
                                        content,
                                        is_error,
                                    } => {
                                        let content_preview = match content {
                                            Some(ToolResultContent::Text(t)) => {
                                                let preview: String = t.chars().take(200).collect();
                                                format!(
                                                    "Text({} chars): {}{}",
                                                    t.len(),
                                                    preview,
                                                    if t.len() > 200 { "..." } else { "" }
                                                )
                                            }
                                            Some(ToolResultContent::Blocks(bs)) => {
                                                format!("Blocks({})", bs.len())
                                            }
                                            None => "None".to_string(),
                                        };
                                        tracing::warn!(
                                            "    [BLK {j}] ToolResult use_id={} is_error={:?} content={}",
                                            tool_use_id,
                                            is_error,
                                            content_preview
                                        );
                                    }
                                    _ => {
                                        tracing::warn!("    [BLK {j}] Other block type");
                                    }
                                }
                            }
                        }
                    }
                }
                tracing::warn!("===== [DEBUG] end of LLM context dump =====");
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

            // Orchestrator owns the stream retry loop so it can emit TaskRetrying
            // events to the frontend between attempts.
            let retry_cfg = crate::llm::retry::RetryConfig::for_stream();
            let max_attempts = retry_cfg.max_retries;
            let mut attempt = 0u32;

            let mut stream = loop {
                attempt += 1;
                let cancel_token = context.create_stream_cancel_token();

                match llm_service
                    .call_stream(llm_request.clone(), cancel_token)
                    .await
                {
                    Ok(s) => {
                        if attempt > 1 {
                            tracing::info!("LLM stream connected after {} retries", attempt - 1);
                        }
                        break s;
                    }
                    Err(e) => {
                        let retryable = e
                            .as_provider()
                            .is_some_and(crate::llm::retry::is_retryable_error);

                        if !retryable || attempt >= max_attempts {
                            return Err(AgentRunError::InternalError(format!(
                                "LLM stream call failed: {e}"
                            )));
                        }

                        let reason = e
                            .as_provider()
                            .map(crate::llm::retry::error_retry_reason)
                            .unwrap_or("non_provider");
                        let delay = retry_cfg.delay_for_attempt(attempt - 1);

                        tracing::warn!(
                            "LLM stream failed (attempt {}/{}): {}. Retry in {:?}",
                            attempt,
                            max_attempts,
                            e,
                            delay
                        );

                        if let Err(err) = context
                            .emit_event(crate::agent::types::AgentRunEvent::AgentRunRetrying {
                                run_id: context.run_id.to_string(),
                                attempt,
                                max_attempts,
                                reason: reason.to_string(),
                                error_message: e.to_string(),
                                retry_in_ms: delay.as_millis() as u64,
                            })
                            .await
                        {
                            warn!("Failed to emit task retrying event: {}", err);
                        }

                        tokio::time::sleep(delay).await;
                    }
                }
            };

            // New stream processing state
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
            let mut stop_reason: Option<crate::llm::anthropic_types::StopReason> = None;

            // ===== Phase 3: Process Anthropic StreamEvent =====
            while let Some(item) = stream.next().await {
                if context.is_aborted() {
                    return Err(AgentRunError::TaskInterrupted);
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
                            let should_render = should_render_tool_block(context, &name).await;
                            tool_block_visibility.insert(id.clone(), should_render);
                            if should_render {
                                let now = Utc::now();
                                tool_block_started_at.insert(id.clone(), now);
                                context
                                    .assistant_append_block(Block::Tool(ToolBlock {
                                        id: id.clone(),
                                        call_id: id.clone(),
                                        name: name.clone(),
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
                        }
                        ContentBlockStart::Thinking { thinking, metadata } => {
                            if !thinking.is_empty() {
                                iter_ctx.append_thinking(&thinking);
                            }
                            current_blocks
                                .insert(index, BlockAccumulator::Thinking { thinking, metadata });
                        }
                        ContentBlockStart::Unknown => {}
                    },
                    Ok(StreamEvent::ContentBlockDelta { index, delta }) => {
                        if let Some(block) = current_blocks.get_mut(&index) {
                            match delta {
                                ContentDelta::Text { text } => {
                                    if let BlockAccumulator::Text(s) = block {
                                        s.push_str(&text);
                                        let id = text_stream_id
                                            .get_or_insert_with(|| Uuid::new_v4().to_string())
                                            .clone();
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

                                        if !tool_block_visibility.get(id).copied().unwrap_or(false)
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

                                        let started_at = match tool_block_started_at.get(id) {
                                            Some(started_at) => *started_at,
                                            None => {
                                                warn!(
                                                    tool_use_id = %id,
                                                    tool_name = %name,
                                                    "Missing tool start timestamp for streaming update"
                                                );
                                                Utc::now()
                                            }
                                        };
                                        if let Err(err) = context
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
                                            .await
                                        {
                                            warn!(
                                                "Failed to stream pending tool block '{}' to UI: {}",
                                                id, err
                                            );
                                        }
                                    }
                                }
                                ContentDelta::Thinking { thinking: delta } => {
                                    if let BlockAccumulator::Thinking { thinking, .. } = block {
                                        thinking.push_str(&delta);
                                        let id = thinking_stream_id
                                            .get_or_insert_with(|| Uuid::new_v4().to_string());
                                        let block = Block::Thinking(ThinkingBlock {
                                            id: id.clone(),
                                            content: thinking.clone(),
                                            is_streaming: true,
                                            metadata: None,
                                        });
                                        if thinking_created {
                                            context.assistant_update_block(id, block).await?;
                                        } else {
                                            context.assistant_append_block(block).await?;
                                            thinking_created = true;
                                        }
                                        iter_ctx.append_thinking(&delta);
                                    }
                                }
                                ContentDelta::Signature { signature: sig } => {
                                    if let BlockAccumulator::Thinking { metadata, .. } = block {
                                        let m = metadata.get_or_insert_with(Default::default);
                                        m.signature = Some(sig);
                                    }
                                }
                                ContentDelta::Unknown => {}
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
                                            if let Err(err) =
                                                context.assistant_update_block(id, block).await
                                            {
                                                warn!(
                                                    "Failed to finalize text block '{}' in UI: {}",
                                                    id, err
                                                );
                                            }
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
                                    ..
                                } => {
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
                                                AgentRunError::InternalError(
                                                    "Empty tool input JSON from stream".to_string(),
                                                )
                                            })?
                                            .map_err(|err| {
                                                AgentRunError::InternalError(format!(
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
                                        let started_at =
                                            tool_block_started_at.get(&id).cloned().unwrap_or(now);
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
                                BlockAccumulator::Thinking { thinking, metadata } => {
                                    // Update UI with final thinking content
                                    if thinking_created {
                                        if let Some(id) = &thinking_stream_id {
                                            let block = Block::Thinking(ThinkingBlock {
                                                id: id.clone(),
                                                content: thinking.clone(),
                                                is_streaming: false,
                                                metadata: metadata.clone(),
                                            });
                                            if let Err(err) =
                                                context.assistant_update_block(id, block).await
                                            {
                                                warn!(
                                                    "Failed to finalize thinking block '{}' in UI: {}",
                                                    id, err
                                                );
                                            }
                                        }
                                    }

                                    let signature =
                                        metadata.as_ref().and_then(|m| m.signature.clone());

                                    if let Some(ref meta) = metadata {
                                        tracing::debug!(
                                            "Reasoning metadata: item_id={:?}, provider={:?}",
                                            meta.item_id,
                                            meta.provider
                                        );
                                    }

                                    tool_use_blocks.push(ContentBlock::Thinking {
                                        thinking,
                                        signature,
                                        reasoning_metadata: metadata,
                                    });
                                }
                            }
                        }
                    }
                    Ok(StreamEvent::MessageDelta {
                        delta,
                        usage: _usage,
                    }) => {
                        if let Some(reason) = delta.stop_reason {
                            stop_reason = Some(reason);
                        }
                    }
                    Ok(StreamEvent::MessageStop) => {
                        break;
                    }
                    Ok(StreamEvent::Ping) => {}
                    Ok(StreamEvent::Error { error }) => {
                        return Err(AgentRunError::InternalError(error.message));
                    }
                    Ok(StreamEvent::Unknown) => {}
                    Err(e) => {
                        return Err(AgentRunError::InternalError(e.to_string()));
                    }
                }
            }

            if context.is_aborted() {
                return Err(AgentRunError::TaskInterrupted);
            }

            // ===== Phase 4: Write accumulated content to context =====
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

                        let note = if fabricated_tool_output_count == 1 {
                            "⚠️ Invalid tool output detected. Retrying..."
                        } else {
                            "⚠️ Invalid tool output detected again. Aborting."
                        };
                        tracing::warn!("{}", note);

                        if fabricated_tool_output_count == 1 {
                            continue;
                        }

                        return Err(AgentRunError::InternalError(format!(
                            "LLM output contained fabricated tool results (count={fabricated_tool_output_count})"
                        )));
                    }
                }
            }

            context
                .add_assistant_message(final_text.clone(), Some(tool_use_blocks))
                .await?;

            // ===== Phase 5: Classify iteration outcome =====
            let final_thinking = {
                let t = iter_ctx.thinking();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            };

            // Follow opencode-dev logic: only complete when stop_reason is NOT tool_use
            let outcome = if !pending_tool_calls.is_empty() {
                IterationOutcome::ContinueWithTools {
                    tool_calls: pending_tool_calls.clone(),
                }
            } else {
                use crate::llm::anthropic_types::StopReason;
                match stop_reason {
                    Some(StopReason::ToolUse) => {
                        // Tool use without pending calls - shouldn't happen, but treat as empty
                        IterationOutcome::Empty
                    }
                    Some(StopReason::EndTurn)
                    | Some(StopReason::MaxTokens)
                    | Some(StopReason::StopSequence)
                    | None => {
                        // Task complete: LLM ended turn or reached limits
                        if final_text.as_deref().is_some_and(|s| !s.trim().is_empty())
                            || final_thinking.is_some()
                        {
                            IterationOutcome::Complete {
                                thinking: final_thinking,
                                output: final_text.clone(),
                            }
                        } else {
                            IterationOutcome::Empty
                        }
                    }
                }
            };

            // ===== Phase 6: Execute actions based on outcome =====
            if !outcome.is_empty() {
                empty_response_count = 0;
            }
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
                                let started_at =
                                    tool_block_started_at.get(call_id).cloned().unwrap_or(now);
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
                        if let Err(err) = context
                            .set_system_prompt_overlay(Some(
                                crate::llm::anthropic_types::SystemPrompt::Text(format!(
                                    "<system-reminder type=\"duplicate-tools\">\n{warning}\n</system-reminder>"
                                )),
                            ))
                            .await
                        {
                            warn!("Failed to apply duplicate-tools reminder overlay: {}", err);
                        }
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
                        if let Err(err) = context
                            .set_system_prompt_overlay(Some(
                                crate::llm::anthropic_types::SystemPrompt::Text(loop_warning),
                            ))
                            .await
                        {
                            warn!("Failed to set loop-detection prompt overlay: {}", err);
                        }
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
                    empty_response_count = empty_response_count.saturating_add(1);

                    if empty_response_count == 1 {
                        warn!("Iteration {}: empty response - retrying once", iteration);
                        if let Err(err) = context
                            .set_system_prompt_overlay(Some(
                                crate::llm::anthropic_types::SystemPrompt::Text(
                                    "<system-reminder type=\"empty-response\">\nYou returned an empty response (no text and no tool calls). Please either (1) call an appropriate tool, or (2) provide a short text response describing your next step.\n</system-reminder>"
                                        .to_string(),
                                ),
                            ))
                            .await
                        {
                            warn!("Failed to set empty-response prompt overlay: {}", err);
                        }

                        let snapshot = iter_ctx.finalize();
                        Self::update_session_stats(context, &snapshot).await;
                        continue;
                    }

                    warn!("Iteration {}: empty response - terminating", iteration);
                    tracing::warn!(
                        "⚠️  Empty LLM response at iteration {}, terminating task",
                        iteration
                    );
                    let snapshot = iter_ctx.finalize();
                    Self::update_session_stats(context, &snapshot).await;
                    break;
                }
            }
        }

        tracing::info!("🏁 ReAct loop finished");
        Ok(())
    }

    async fn update_session_stats(context: &AgentRunContext, snapshot: &IterationSnapshot) {
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
        context: &AgentRunContext,
        model_id: &str,
        context_window: u32,
    ) -> AgentRunResult<()> {
        let service = CompactionService::new(
            Arc::clone(&self.database),
            Arc::clone(&self.agent_persistence),
            CompactionConfig::default(),
        );

        let prepared = service
            .prepare_compaction(context.thread_id, context_window, CompactionTrigger::Auto)
            .await
            .map_err(|e| AgentRunError::InternalError(e.to_string()))?;

        let Some(job) = prepared.summary_job else {
            return Ok(());
        };

        context
            .emit_event(crate::agent::types::AgentRunEvent::MessageCreated {
                run_id: context.run_id.to_string(),
                message: job.summary_message.clone(),
            })
            .await?;

        let completed = service
            .complete_summary_job(job, model_id)
            .await
            .map_err(|e| AgentRunError::InternalError(e.to_string()))?;

        let context_usage = context.calculate_context_usage(model_id).await;
        context
            .emit_event(crate::agent::types::AgentRunEvent::MessageFinished {
                run_id: context.run_id.to_string(),
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
                || lower.contains("完成")  // "completed" in Chinese
                || lower.contains("成功")  // "successfully" in Chinese
                || lower.contains("失败")  // "failed" in Chinese
                || lower.contains("错误"))
        // "error" in Chinese
        {
            return true;
        }
    }
    false
}

/// Build developer-context sections describing active subagents.
///
/// Codex injects this as part of `EnvironmentContext` (developer context layer),
/// not as a conversation message. The model sees it as authoritative environment
/// state that persists across compaction and doesn't pollute the message stream.
async fn build_subagents_developer_context(
    database: &DatabaseManager,
    parent_thread_id: i64,
) -> Vec<String> {
    let rows = match sqlx::query(
        "SELECT name, profile, task_title, status, latest_activity, final_summary, error_message
         FROM subagents
         WHERE parent_thread_id = ?
         ORDER BY created_at ASC, id ASC",
    )
    .bind(parent_thread_id)
    .fetch_all(database.pool())
    .await
    {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    if rows.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    lines.push("<subagents>".to_string());
    for row in &rows {
        let name: String = row.try_get("name").unwrap_or_default();
        let profile: String = row.try_get("profile").unwrap_or_default();
        let task_title: String = row.try_get("task_title").unwrap_or_default();
        let status: String = row.try_get("status").unwrap_or_default();
        let latest_activity: Option<String> = row.try_get("latest_activity").ok().flatten();
        let final_summary: Option<String> = row.try_get("final_summary").ok().flatten();
        let error_message: Option<String> = row.try_get("error_message").ok().flatten();
        lines.push(format!("  - {}: {}", name.trim(), task_title.trim()));
        lines.push(format!("    profile={} status={}", profile, status));
        if let Some(summary) = final_summary
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("    summary={summary}"));
        } else if let Some(activity) = latest_activity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("    activity={activity}"));
        } else if let Some(error) = error_message
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("    error={error}"));
        }
    }
    lines.push("</subagents>".to_string());

    vec![lines.join("\n")]
}

// Compaction business rules live in `agent/compaction/*` (not in the orchestrator).
