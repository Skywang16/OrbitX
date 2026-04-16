use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{path::Path, path::PathBuf};

use dashmap::{mapref::entry::Entry, DashMap};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use uuid::Uuid;

use super::metadata::{RateLimitConfig, ToolCategory, ToolMetadata};
use super::r#trait::{
    RunnableTool, ToolAvailabilityContext, ToolDescriptionContext, ToolResult, ToolResultContent,
    ToolResultStatus, ToolSchema,
};
use crate::agent::common::truncate_chars;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::builtin::file_utils::{ensure_absolute, normalize_path};
use crate::agent::types::AgentRunEvent;
use crate::agent::{
    permissions::PermissionChecker, permissions::PermissionDecision, permissions::ToolAction,
    permissions::ToolFilter,
};

struct RateLimiter {
    calls: Vec<Instant>,
    config: RateLimitConfig,
}

impl RateLimiter {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            calls: Vec::new(),
            config,
        }
    }

    fn check_and_record(&mut self, tool_name: &str) -> ToolExecutorResult<()> {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        self.calls
            .retain(|&call_time| now.duration_since(call_time) < window);

        if self.calls.len() >= self.config.max_calls as usize {
            return Err(ToolExecutorError::ResourceLimitExceeded {
                tool_name: tool_name.to_string(),
                resource_type: format!(
                    "rate limit exceeded ({} calls / {}s)",
                    self.config.max_calls, self.config.window_secs
                ),
            });
        }

        self.calls.push(now);
        Ok(())
    }
}

struct ToolEntry {
    tool: Arc<dyn RunnableTool>,
    metadata: ToolMetadata,
    rate_limiter: Option<Mutex<RateLimiter>>,
    stats: Mutex<ToolExecutionStats>,
}

impl ToolEntry {
    fn new(tool: Arc<dyn RunnableTool>, metadata: ToolMetadata) -> Self {
        let rate_limiter = metadata
            .rate_limit
            .clone()
            .map(|cfg| Mutex::new(RateLimiter::new(cfg)));
        Self {
            tool,
            metadata,
            rate_limiter,
            stats: Mutex::new(ToolExecutionStats::default()),
        }
    }
}

pub struct ToolRegistry {
    aliases: DashMap<String, String>,
    entries: DashMap<String, ToolEntry>,
    settings_permissions: Option<Arc<PermissionChecker>>,
    /// Agent tool filter: whitelist/blacklist for tool visibility.
    /// Separate from settings_permissions (which controls allow/deny/ask confirmation).
    agent_tool_filter: Option<Arc<ToolFilter>>,
    confirmations: Arc<ToolConfirmationManager>,
}

/// Global (per-process) confirmation queue/state.
///
/// Multiple ToolRegistry instances exist in multi-agent/task-execution mode; the UI only supports one
/// confirmation dialog at a time, so confirmation state must be shared to avoid deadlocks.
pub struct ToolConfirmationManager {
    pending_confirmations: DashMap<String, PendingConfirmation>,
    confirmation_state: tokio::sync::Mutex<ConfirmationState>,
}

struct PendingConfirmation {
    tx: tokio::sync::oneshot::Sender<ToolConfirmationDecision>,
    run_id: String,
    workspace_path: String,
    tool_name: String,
    summary: String,
    permission: String,
    always_patterns: Vec<String>,
}

#[derive(Debug, Default)]
struct ConfirmationState {
    active_request_id: Option<String>,
    queue: VecDeque<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolExecutionStats {
    pub total_calls: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: u64,
    pub last_called_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ToolRegistry {
    /// Only constructor - explicitly pass permissions
    pub fn new(
        settings_permissions: Option<Arc<PermissionChecker>>,
        agent_tool_filter: Option<Arc<ToolFilter>>,
        confirmations: Arc<ToolConfirmationManager>,
    ) -> Self {
        Self {
            aliases: DashMap::new(),
            entries: DashMap::new(),
            settings_permissions,
            agent_tool_filter,
            confirmations,
        }
    }

    pub async fn resolve_confirmation(
        &self,
        context: &AgentRunContext,
        request_id: &str,
        decision: ToolConfirmationDecision,
    ) -> bool {
        tracing::info!(
            "🔐 [resolve] request_id={} decision={:?} pending_count={}",
            request_id,
            decision,
            self.confirmations.pending_confirmations.len()
        );
        let removed = self.confirmations.pending_confirmations.remove(request_id);
        let Some((_, pending)) = removed else {
            tracing::warn!(
                "🔐 [resolve] request_id={} NOT FOUND in pending_confirmations! Keys: {:?}",
                request_id,
                self.confirmations
                    .pending_confirmations
                    .iter()
                    .map(|e| e.key().clone())
                    .collect::<Vec<_>>()
            );
            return false;
        };

        let workspace = pending.workspace_path.clone();
        let run_id = pending.run_id.clone();
        let permission = pending.permission.clone();
        let always_patterns = pending.always_patterns.clone();
        tracing::info!(
            "🔐 [resolve] delivering decision to oneshot: request_id={} run_id={} tool={}",
            request_id,
            run_id,
            pending.tool_name
        );

        let ok = pending.tx.send(decision).is_ok();
        if !ok {
            warn!(
                "🔐 [resolve] Failed to deliver tool confirmation decision for request '{}' (receiver dropped)",
                request_id
            );
        } else {
            tracing::info!(
                "🔐 [resolve] decision delivered successfully: request_id={}",
                request_id
            );
        }

        match decision {
            ToolConfirmationDecision::AllowAlways => {
                let settings_mgr = context.settings_manager();
                let workspace_root = std::path::PathBuf::from(&workspace);
                if let Err(err) = persist_approval_rules_to_local_settings(
                    &settings_mgr,
                    &workspace_root,
                    &permission,
                    &always_patterns,
                )
                .await
                {
                    warn!("Failed to persist tool approval rules: {}", err);
                }
                cascade_approvals_from_local_settings(
                    &settings_mgr,
                    &workspace_root,
                    &self.confirmations.pending_confirmations,
                )
                .await;
            }
            ToolConfirmationDecision::AllowOnce => {
                self.cascade_allow_once(&run_id, &workspace, &permission, &always_patterns);
            }
            ToolConfirmationDecision::Deny => {
                self.cancel_pending_confirmations_for_task(context, &run_id)
                    .await;
            }
        }

        self.finish_confirmation_and_pump_next(context, request_id)
            .await;

        ok
    }

    pub async fn cancel_pending_confirmations_for_task(
        &self,
        context: &AgentRunContext,
        run_id: &str,
    ) {
        let to_cancel = self
            .confirmations
            .pending_confirmations
            .iter()
            .filter(|entry| entry.value().run_id == run_id)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();

        for request_id in to_cancel {
            self.drop_pending_confirmation(&request_id, ToolConfirmationDecision::Deny)
                .await;
        }

        self.pump_next_confirmation(context).await;
    }

    fn cascade_allow_once(
        &self,
        run_id: &str,
        workspace_path: &str,
        permission: &str,
        always_patterns: &[String],
    ) {
        let mut to_resolve = Vec::new();
        for entry in self.confirmations.pending_confirmations.iter() {
            let id = entry.key().clone();
            let p = entry.value();
            if p.run_id != run_id {
                continue;
            }
            if p.workspace_path != workspace_path {
                continue;
            }
            if p.permission != permission {
                continue;
            }
            // For shell commands: cascade ALL pending shell confirmations in the same task.
            // For other tools: require the patterns to match exactly (same file/url/etc.).
            let should_cascade = if permission == "shell" {
                true
            } else {
                p.always_patterns == always_patterns
            };
            if !should_cascade {
                continue;
            }
            to_resolve.push(id);
        }

        for id in to_resolve {
            if let Some((_, pending)) = self.confirmations.pending_confirmations.remove(&id) {
                if pending
                    .tx
                    .send(ToolConfirmationDecision::AllowOnce)
                    .is_err()
                {
                    warn!(
                        "Failed to cascade allow-once tool confirmation for request '{}'",
                        id
                    );
                }
            }
        }
    }

    async fn finish_confirmation_and_pump_next(&self, context: &AgentRunContext, request_id: &str) {
        let mut state = self.confirmations.confirmation_state.lock().await;

        if state.active_request_id.as_deref() == Some(request_id) {
            tracing::info!("🔐 [pump] clearing active request: {}", request_id);
            state.active_request_id = None;
        } else {
            tracing::info!(
                "🔐 [pump] request {} not active (active={:?}), removing from queue",
                request_id,
                state.active_request_id
            );
            // If it was queued (shouldn't happen with a single-dialog UI), drop it.
            state.queue.retain(|id| id != request_id);
        }

        drop(state);
        self.pump_next_confirmation(context).await;
    }

    async fn pump_next_confirmation(&self, context: &AgentRunContext) {
        let mut state = self.confirmations.confirmation_state.lock().await;

        // Only pump when there is no active request.
        if state.active_request_id.is_some() {
            tracing::debug!(
                "🔐 [pump] skipping pump, active request exists: {:?}",
                state.active_request_id
            );
            return;
        }

        let next = loop {
            let Some(candidate) = state.queue.pop_front() else {
                break None;
            };
            if self
                .confirmations
                .pending_confirmations
                .contains_key(&candidate)
            {
                break Some(candidate);
            }
            tracing::debug!("🔐 [pump] skipping stale queued request: {}", candidate);
        };

        let Some(next_id) = next else {
            tracing::debug!("🔐 [pump] queue empty, nothing to pump");
            return;
        };

        tracing::info!("🔐 [pump] activating next request: {}", next_id);
        state.active_request_id = Some(next_id.clone());
        drop(state);

        // Best effort: if UI is unavailable, don't wedge the queue forever.
        if let Err(err) = self.emit_confirmation_request(context, &next_id).await {
            warn!("Failed to emit confirmation request: {}", err);
            self.drop_pending_confirmation(&next_id, ToolConfirmationDecision::Deny)
                .await;
        }
    }

    async fn drop_pending_confirmation(
        &self,
        request_id: &str,
        decision: ToolConfirmationDecision,
    ) {
        if let Some((_, pending)) = self.confirmations.pending_confirmations.remove(request_id) {
            if pending.tx.send(decision).is_err() {
                warn!(
                    "Failed to deliver dropped tool confirmation for request '{}'",
                    request_id
                );
            }
        }
        let mut state = self.confirmations.confirmation_state.lock().await;
        if state.active_request_id.as_deref() == Some(request_id) {
            state.active_request_id = None;
        }
        state.queue.retain(|id| id != request_id);
    }

    async fn emit_confirmation_request(
        &self,
        context: &AgentRunContext,
        request_id: &str,
    ) -> ToolExecutorResult<()> {
        let pending = self
            .confirmations
            .pending_confirmations
            .get(request_id)
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "tool_confirmation".to_string(),
                error: "Pending confirmation not found".to_string(),
            })?;

        context
            .emit_event(AgentRunEvent::ToolConfirmationRequested {
                run_id: pending.run_id.clone(),
                request_id: request_id.to_string(),
                workspace_path: pending.workspace_path.clone(),
                tool_name: pending.tool_name.clone(),
                summary: pending.summary.clone(),
            })
            .await
            .map_err(|err| ToolExecutorError::ExecutionFailed {
                tool_name: pending.tool_name.clone(),
                error: format!(
                    "Failed to request user confirmation (UI channel unavailable): {err}"
                ),
            })?;

        Ok(())
    }

    pub async fn register(
        &self,
        name: &str,
        tool: Arc<dyn RunnableTool>,
        is_chat_mode: bool,
        availability_ctx: &ToolAvailabilityContext,
    ) -> ToolExecutorResult<()> {
        // Check tool availability first
        if !tool.is_available(availability_ctx) {
            return Ok(()); // Skip unavailable tools silently
        }

        let key = name.to_string();
        let metadata = tool.metadata();

        // === Chat mode tool filtering logic ===
        if is_chat_mode {
            // Blacklist: prohibit FileWrite and Execution categories
            match metadata.category {
                ToolCategory::FileWrite | ToolCategory::Execution => {
                    return Ok(()); // Silently skip, do not register
                }
                // Whitelist: allow read-only tools
                ToolCategory::FileRead | ToolCategory::CodeAnalysis | ToolCategory::FileSystem => {
                    // Directly allow, no permission check needed
                }
                // Other categories: check permissions
                _ => {
                    // permissions are enforced at runtime via settings.json (allow/deny/ask)
                }
            }
        } else {
            // Agent mode: permissions are enforced at runtime via settings.json (allow/deny/ask)
        }

        match self.entries.entry(key) {
            Entry::Occupied(_) => {
                return Err(ToolExecutorError::ConfigurationError(format!(
                    "Tool already registered: {name}"
                )));
            }
            Entry::Vacant(entry) => {
                entry.insert(ToolEntry::new(tool, metadata));
            }
        }

        Ok(())
    }

    async fn resolve_name(&self, name: &str) -> Option<String> {
        if self.entries.contains_key(name) {
            return Some(name.to_string());
        }

        self.aliases.get(name).map(|entry| entry.clone())
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn RunnableTool>> {
        let resolved = self.resolve_name(name).await?;
        self.entries
            .get(&resolved)
            .map(|entry| Arc::clone(&entry.value().tool))
    }

    pub async fn get_tool_metadata(&self, name: &str) -> Option<ToolMetadata> {
        let resolved = self.resolve_name(name).await?;
        self.entries
            .get(&resolved)
            .map(|entry| entry.value().metadata.clone())
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolResult {
        let start = Instant::now();

        let resolved = match self.resolve_name(tool_name).await {
            Some(name) => name,
            None => {
                warn!("🚫 Tool not found: {}", tool_name);
                return self
                    .make_error_result(
                        tool_name,
                        "Tool not found".to_string(),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        let metadata = match self.get_tool_metadata(&resolved).await {
            Some(meta) => meta,
            None => {
                warn!("🚫 Tool metadata not found: {}", resolved);
                return self
                    .make_error_result(
                        &resolved,
                        "Tool not found".to_string(),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        let action = build_tool_action(&resolved, &metadata, context, &args);
        let (settings_decision, settings_matched) =
            if let Some(checker) = self.settings_permissions.as_ref() {
                let (decision, matched) = checker.check_with_match(&action);
                (Some(decision), matched)
            } else {
                (None, false)
            };
        // Good taste: "no matching rule" is not the same thing as "Ask".
        // If the user's allow/deny/ask lists don't match this action, treat it as "no decision"
        // and let tool metadata + workspace boundary checks drive whether we prompt.
        let settings_decision = if settings_matched {
            settings_decision
        } else {
            None
        };

        // Agent tool filter: check if the tool is visible to this agent.
        // This is a simple yes/no check, not a deny/ask/allow decision.
        let agent_blocked = self
            .agent_tool_filter
            .as_ref()
            .is_some_and(|filter| !filter.is_allowed(&resolved));

        if agent_blocked {
            return self
                .make_error_result(
                    &resolved,
                    format!("Tool not available for this agent: {resolved}"),
                    Some(format!("action={} source=agent_tool_filter", action.tool)),
                    ToolResultStatus::Error,
                    Some("denied".to_string()),
                    start,
                )
                .await;
        }

        if matches!(settings_decision, Some(PermissionDecision::Deny)) {
            return self
                .make_error_result(
                    &resolved,
                    format!("Denied by settings permissions: {resolved}"),
                    Some(format!("action={} source=settings.json", action.tool)),
                    ToolResultStatus::Error,
                    Some("denied".to_string()),
                    start,
                )
                .await;
        }

        if let Err(err) = self.check_rate_limit(&resolved).await {
            let detail = Some(format!(
                "category={}, priority={}",
                metadata.category.as_str(),
                metadata.priority.as_str()
            ));
            return self
                .make_error_result(
                    &resolved,
                    err.to_string(),
                    detail,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        let requires_confirmation = match settings_decision {
            // Collab tools are orchestration, not direct side-effecting tools. They should not be
            // blocked by confirmation prompts unless explicitly denied by settings.
            _ if is_collab_tool(&resolved)
                && !matches!(settings_decision, Some(PermissionDecision::Deny)) =>
            {
                false
            }
            Some(PermissionDecision::Allow) => false,
            Some(PermissionDecision::Ask) => true,
            Some(PermissionDecision::Deny) => true, // already handled above, unreachable
            None => {
                metadata.requires_confirmation
                    || self
                        .requires_workspace_confirmation(&metadata, context, &args)
                        .await
            }
        };

        if requires_confirmation {
            tracing::info!(
                "⏸️  Waiting for confirmation: {} (run_id={}, settings_decision={:?}, matched={})",
                resolved,
                context.run_id,
                settings_decision,
                settings_matched
            );
            if let Some(blocked) = self
                .confirm_or_block_tool(&resolved, &metadata, context, &args, &action, start)
                .await
            {
                tracing::info!("🚫 Tool denied: {}", resolved);
                return blocked;
            }
            tracing::info!("✅ Tool confirmed: {}", resolved);
        } else {
            tracing::debug!(
                "🔓 No confirmation needed: {} (settings_decision={:?}, matched={}, meta_confirm={}, category={:?})",
                resolved, settings_decision, settings_matched, metadata.requires_confirmation, metadata.category
            );
        }

        let timeout = metadata.effective_timeout();

        let timeout_result = tokio::time::timeout(
            timeout,
            self.execute_tool_impl(&resolved, context, args, start),
        )
        .await;

        match timeout_result {
            Ok(result) => result,
            Err(_) => {
                let elapsed = start.elapsed().as_millis() as u64;
                self.update_stats(&resolved, false, elapsed).await;
                error!("Tool {} timed out {:?}", resolved, timeout);

                ToolResult {
                    content: vec![ToolResultContent::Error(format!(
                        "Tool {} timed out (timeout={:?}, priority={})",
                        resolved,
                        timeout,
                        metadata.priority.as_str()
                    ))],
                    status: ToolResultStatus::Error,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed),
                    ext_info: None,
                }
            }
        }
    }

    fn effective_permission_decision(
        &self,
        tool_name: &str,
        action: &ToolAction,
    ) -> PermissionDecision {
        // Agent tool filter: if tool is not allowed, treat as Deny
        if self
            .agent_tool_filter
            .as_ref()
            .is_some_and(|filter| !filter.is_allowed(tool_name))
        {
            return PermissionDecision::Deny;
        }

        match self.settings_permissions.as_ref() {
            Some(checker) => checker.check(action),
            None => PermissionDecision::Allow,
        }
    }

    async fn check_rate_limit(&self, tool_name: &str) -> ToolExecutorResult<()> {
        if let Some(entry) = self.entries.get(tool_name) {
            if let Some(limiter) = &entry.value().rate_limiter {
                limiter.lock().check_and_record(tool_name)?;
            }
        }
        Ok(())
    }

    async fn requires_workspace_confirmation(
        &self,
        metadata: &ToolMetadata,
        context: &AgentRunContext,
        args: &serde_json::Value,
    ) -> bool {
        // Only write operations need workspace boundary confirmation.
        // Read-only tools (FileRead, FileSystem, CodeAnalysis) should never require confirmation.
        if !matches!(metadata.category, ToolCategory::FileWrite) {
            return false;
        }

        let Some(path) = tool_path_arg(args, metadata) else {
            return false;
        };

        let resolved_path = match ensure_absolute(&path, &context.cwd) {
            Ok(path) => path,
            Err(err) => {
                warn!(
                    "Failed to resolve tool path for workspace confirmation (tool={}, path={}): {}",
                    metadata.category.as_str(),
                    path,
                    err
                );
                return false;
            }
        };

        let workspace_root = PathBuf::from(context.cwd.as_ref());
        if !workspace_root.is_absolute() {
            return false;
        }

        !is_within_workspace(&workspace_root, &resolved_path).await
    }

    async fn confirm_or_block_tool(
        &self,
        tool_name: &str,
        metadata: &ToolMetadata,
        context: &AgentRunContext,
        args: &serde_json::Value,
        action: &ToolAction,
        start: Instant,
    ) -> Option<ToolResult> {
        if context.is_aborted() {
            return Some(
                self.make_error_result(
                    tool_name,
                    "Task aborted; tool execution cancelled".to_string(),
                    None,
                    ToolResultStatus::Cancelled,
                    Some("aborted".to_string()),
                    start,
                )
                .await,
            );
        }

        let workspace = context.session().workspace.to_string_lossy().to_string();
        let (permission, always_patterns) = confirmation_scope(action, metadata);

        let settings_mgr = context.settings_manager();
        let workspace_root = std::path::PathBuf::from(&workspace);

        if let Some(ext) = external_directory_always_patterns(metadata, context, args).await {
            if !is_preapproved_in_local_settings(
                &settings_mgr,
                &workspace_root,
                "external_directory",
                &ext,
            )
            .await
            {
                let summary = summarize_tool_call(tool_name, metadata, args);
                let decision = match self
                    .request_tool_confirmation(
                        context,
                        &workspace,
                        "external_directory",
                        &format!("external directory access required: {summary}"),
                        "external_directory",
                        &ext,
                    )
                    .await
                {
                    Ok(d) => d,
                    Err(err) => {
                        return Some(
                            self.make_error_result(
                                tool_name,
                                err.to_string(),
                                Some("tool_confirmation".into()),
                                ToolResultStatus::Cancelled,
                                Some("confirmation_failed".to_string()),
                                start,
                            )
                            .await,
                        );
                    }
                };

                if matches!(decision, ToolConfirmationDecision::Deny) {
                    return Some(
                        self.make_error_result(
                            tool_name,
                            format!("User denied external directory access for: {tool_name}"),
                            Some(summary),
                            ToolResultStatus::Cancelled,
                            Some("denied".to_string()),
                            start,
                        )
                        .await,
                    );
                }
            }
        }

        if is_preapproved_in_local_settings(
            &settings_mgr,
            &workspace_root,
            &permission,
            &always_patterns,
        )
        .await
        {
            tracing::info!(
                "🔐 [confirm] pre-approved: tool={} permission={} patterns={:?}",
                tool_name,
                permission,
                always_patterns
            );
            return None;
        }

        let summary = summarize_tool_call(tool_name, metadata, args);
        let decision = match self
            .request_tool_confirmation(
                context,
                &workspace,
                tool_name,
                &summary,
                &permission,
                &always_patterns,
            )
            .await
        {
            Ok(d) => d,
            Err(err) => {
                return Some(
                    self.make_error_result(
                        tool_name,
                        err.to_string(),
                        Some("tool_confirmation".into()),
                        ToolResultStatus::Cancelled,
                        Some("confirmation_failed".to_string()),
                        start,
                    )
                    .await,
                );
            }
        };

        match decision {
            ToolConfirmationDecision::AllowOnce => None,
            ToolConfirmationDecision::AllowAlways => None,
            ToolConfirmationDecision::Deny => Some(
                self.make_error_result(
                    tool_name,
                    format!("User denied tool execution: {tool_name}"),
                    Some(summary),
                    ToolResultStatus::Cancelled,
                    Some("denied".to_string()),
                    start,
                )
                .await,
            ),
        }
    }

    async fn request_tool_confirmation(
        &self,
        context: &AgentRunContext,
        workspace_path: &str,
        tool_name: &str,
        summary: &str,
        permission: &str,
        always_patterns: &[String],
    ) -> ToolExecutorResult<ToolConfirmationDecision> {
        let request_id = Uuid::new_v4().to_string();
        tracing::info!(
            "🔐 [confirm] creating request: id={} tool={} run_id={} permission={} patterns={:?}",
            request_id,
            tool_name,
            context.run_id,
            permission,
            always_patterns
        );
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.confirmations.pending_confirmations.insert(
            request_id.clone(),
            PendingConfirmation {
                tx,
                run_id: context.run_id.to_string(),
                workspace_path: workspace_path.to_string(),
                tool_name: tool_name.to_string(),
                summary: summary.to_string(),
                permission: permission.to_string(),
                always_patterns: always_patterns.to_vec(),
            },
        );

        // UI only supports one active confirmation dialog reliably.
        // Queue confirmations and pump them one by one, so parallel tools don't deadlock.
        let should_emit = {
            let mut state = self.confirmations.confirmation_state.lock().await;
            if state.active_request_id.is_none() {
                state.active_request_id = Some(request_id.clone());
                tracing::info!(
                    "🔐 [confirm] emitting immediately: id={} (no active request)",
                    request_id
                );
                true
            } else {
                tracing::info!(
                    "🔐 [confirm] queued: id={} (active={:?}, queue_len={})",
                    request_id,
                    state.active_request_id,
                    state.queue.len()
                );
                state.queue.push_back(request_id.clone());
                false
            }
        };

        if should_emit {
            if let Err(err) = self.emit_confirmation_request(context, &request_id).await {
                tracing::warn!("🔐 [confirm] emit failed: id={} err={}", request_id, err);
                self.confirmations.pending_confirmations.remove(&request_id);
                self.finish_confirmation_and_pump_next(context, &request_id)
                    .await;
                return Err(err);
            }
        }

        tracing::info!(
            "🔐 [confirm] waiting for user decision: id={} tool={}",
            request_id,
            tool_name
        );
        let decision = tokio::select! {
            res = tokio::time::timeout(Duration::from_secs(600), rx) => {
                match res {
                    Ok(Ok(d)) => {
                        tracing::info!("🔐 [confirm] received decision: id={} decision={:?}", request_id, d);
                        Ok(d)
                    },
                    Ok(Err(_)) => {
                        tracing::warn!("🔐 [confirm] oneshot channel closed (sender dropped): id={}", request_id);
                        Err(ToolExecutorError::ExecutionFailed {
                            tool_name: tool_name.to_string(),
                            error: "Confirmation channel closed".to_string(),
                        })
                    },
                    Err(_) => {
                        tracing::warn!("🔐 [confirm] timed out after 600s: id={}", request_id);
                        Err(ToolExecutorError::ExecutionTimeout {
                            tool_name: tool_name.to_string(),
                            timeout_seconds: 600,
                        })
                    },
                }
            }
            _ = context.states.abort_token.cancelled() => {
                tracing::warn!("🔐 [confirm] aborted: id={}", request_id);
                Err(ToolExecutorError::ExecutionFailed {
                    tool_name: tool_name.to_string(),
                    error: "Task aborted; confirmation cancelled".to_string(),
                })
            }
        };

        if decision.is_err() {
            tracing::warn!(
                "🔐 [confirm] cleaning up failed confirmation: id={}",
                request_id
            );
            self.confirmations.pending_confirmations.remove(&request_id);
            self.finish_confirmation_and_pump_next(context, &request_id)
                .await;
        }

        decision
    }

    async fn execute_tool_impl(
        &self,
        tool_name: &str,
        context: &AgentRunContext,
        args: serde_json::Value,
        start: Instant,
    ) -> ToolResult {
        let tool = match self.get_tool(tool_name).await {
            Some(t) => t,
            None => {
                warn!("🚫 Tool not found: {}", tool_name);
                return self
                    .make_error_result(
                        tool_name,
                        format!("Tool not found: {tool_name}"),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        if let Err(e) = tool.validate_arguments(&args) {
            warn!("⚠️  Invalid arguments for {}: {}", tool_name, e);
            return self
                .make_error_result(
                    tool_name,
                    format!("Argument validation failed: {e}"),
                    None,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        if let Err(e) = tool.before_run(context, &args).await {
            warn!("⚠️  Pre-run hook failed for {}: {}", tool_name, e);
            return self
                .make_error_result(
                    tool_name,
                    format!("Pre-run hook failed: {e}"),
                    None,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        match tool.run(context, args).await {
            Ok(mut r) => {
                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed > 1000 {
                    tracing::info!(
                        "🔧 {} completed in {:.1}s",
                        tool_name,
                        elapsed as f64 / 1000.0
                    );
                }
                r.execution_time_ms = Some(elapsed);
                self.update_stats(tool_name, true, elapsed).await;

                if let Err(e) = tool.after_run(context, &r).await {
                    warn!("⚠️  Tool {} after_run hook failed: {}", tool_name, e);
                }

                r
            }
            Err(e) => {
                return self
                    .make_error_result(
                        tool_name,
                        e.to_string(),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        }
    }

    async fn make_error_result(
        &self,
        tool_name: &str,
        error_message: String,
        details: Option<String>,
        status: ToolResultStatus,
        cancel_reason: Option<String>,
        start: Instant,
    ) -> ToolResult {
        let elapsed = start.elapsed().as_millis() as u64;
        self.update_stats(tool_name, false, elapsed).await;
        error!("Tool {} failed: {}", tool_name, error_message);

        let full_message = if let Some(d) = details {
            format!("{error_message} ({d})")
        } else {
            error_message
        };

        ToolResult {
            content: vec![ToolResultContent::Error(full_message)],
            status,
            cancel_reason,
            execution_time_ms: Some(elapsed),
            ext_info: None,
        }
    }

    async fn update_stats(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        if let Some(entry) = self.entries.get(tool_name) {
            let mut stats = entry.value().stats.lock();
            stats.total_calls += 1;
            if success {
                stats.success_count += 1;
            } else {
                stats.failure_count += 1;
            }
            stats.total_execution_time_ms += execution_time_ms;
            stats.avg_execution_time_ms = stats.total_execution_time_ms / stats.total_calls.max(1);
            stats.last_called_at = Some(chrono::Utc::now());
        }
    }

    /// Get tool schemas with context-aware descriptions
    pub fn get_tool_schemas_with_context(
        &self,
        context: &ToolDescriptionContext,
    ) -> Vec<ToolSchema> {
        let workspace_root = PathBuf::from(context.cwd.as_str());
        self.entries
            .iter()
            .filter(|entry| {
                let tool_name = entry.value().tool.name();
                if tool_name == "task" && context.allowed_subagent_types.is_empty() {
                    return false;
                }
                let action = build_tool_action_for_prompt(tool_name, workspace_root.clone());

                // Agent tool filter: hide tools not available to this agent
                if self
                    .agent_tool_filter
                    .as_ref()
                    .is_some_and(|filter| !filter.is_allowed(tool_name))
                {
                    return false;
                }

                // Settings permissions: hide denied tools
                if self.effective_permission_decision(tool_name, &action)
                    == PermissionDecision::Deny
                {
                    return false;
                }

                true
            })
            .map(|entry| {
                let tool = &entry.value().tool;
                let description = match tool.description_with_context(context) {
                    Some(description) => description,
                    None => tool.description().to_string(),
                };

                ToolSchema {
                    name: tool.name().to_string(),
                    description,
                    parameters: tool.parameters_schema(),
                }
            })
            .collect()
    }
}

fn build_tool_action_for_prompt(tool_name: &str, workspace_root: PathBuf) -> ToolAction {
    if tool_name.starts_with("mcp__") {
        return ToolAction::new(tool_name, workspace_root, vec![]);
    }

    match tool_name {
        "shell" => ToolAction::new("shell", workspace_root, vec![]),
        "read_file" => ToolAction::new("read", workspace_root, vec![]),
        "write_file" => ToolAction::new("write", workspace_root, vec![]),
        "edit_file" => ToolAction::new("edit", workspace_root, vec![]),
        "multi_edit_file" => ToolAction::new("edit", workspace_root, vec![]),
        "list_files" => ToolAction::new("list", workspace_root, vec![]),
        "grep" => ToolAction::new("grep", workspace_root, vec![]),
        "semantic_search" => ToolAction::new("semantic_search", workspace_root, vec![]),
        "read_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "syntax_diagnostics" => ToolAction::new("syntax_diagnostics", workspace_root, vec![]),
        "todowrite" => ToolAction::new("todowrite", workspace_root, vec![]),
        "task" => ToolAction::new("task", workspace_root, vec![]),
        _ => ToolAction::new(tool_name, workspace_root, vec![]),
    }
}

fn build_tool_action(
    tool_name: &str,
    metadata: &ToolMetadata,
    context: &AgentRunContext,
    args: &serde_json::Value,
) -> ToolAction {
    let workspace_root = PathBuf::from(context.cwd.as_ref());

    if tool_name.starts_with("mcp__") {
        return ToolAction::new(tool_name, workspace_root, vec![]);
    }

    match tool_name {
        "shell" => {
            let command = trimmed_string_arg(args, "command");
            ToolAction::new(
                "shell",
                workspace_root,
                command.map_or_else(Vec::new, |command| bash_param_variants(&command)),
            )
        }
        "read_file" => ToolAction::new(
            "read",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "write_file" => ToolAction::new(
            "write",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "edit_file" => ToolAction::new(
            "edit",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "multi_edit_file" => ToolAction::new(
            "edit",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "list_files" => ToolAction::new(
            "list",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "grep" => ToolAction::new("grep", workspace_root, single_arg_variants(args, "query")),
        "semantic_search" => ToolAction::new("semantic_search", workspace_root, vec![]),
        "web_fetch" => {
            let variants = match trimmed_string_arg(args, "url") {
                Some(url) => web_fetch_variants(&url),
                None => Vec::new(),
            };
            ToolAction::new("web_fetch", workspace_root, variants)
        }
        "web_search" => ToolAction::new(
            "web_search",
            workspace_root,
            single_arg_variants(args, "query"),
        ),
        "read_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "syntax_diagnostics" => ToolAction::new("syntax_diagnostics", workspace_root, vec![]),
        "todowrite" => ToolAction::new("todowrite", workspace_root, vec![]),
        "task" => ToolAction::new("task", workspace_root, single_arg_variants(args, "profile")),
        _ => {
            let variants = match metadata.summary_key_arg {
                Some(key) => single_arg_variants(args, key),
                None => Vec::new(),
            };
            ToolAction::new(tool_name, workspace_root, variants)
        }
    }
}

fn is_collab_tool(tool_name: &str) -> bool {
    tool_name == "task"
}

fn path_variants(
    args: &serde_json::Value,
    metadata: &ToolMetadata,
    context: &AgentRunContext,
) -> Vec<String> {
    let Some(path) = tool_path_arg(args, metadata) else {
        return vec![];
    };

    match ensure_absolute(&path, &context.cwd) {
        Ok(resolved) => vec![resolved.to_string_lossy().to_string()],
        Err(err) => {
            warn!("Failed to resolve tool path variant '{}': {}", path, err);
            Vec::new()
        }
    }
}

fn trimmed_string_arg(args: &serde_json::Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn single_arg_variants(args: &serde_json::Value, key: &str) -> Vec<String> {
    match trimmed_string_arg(args, key) {
        Some(value) => vec![value],
        None => Vec::new(),
    }
}

fn tool_path_arg(args: &serde_json::Value, metadata: &ToolMetadata) -> Option<String> {
    trimmed_string_arg(args, "path").or_else(|| {
        metadata
            .summary_key_arg
            .and_then(|key| trimmed_string_arg(args, key))
    })
}

fn bash_param_variants(command: &str) -> Vec<String> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return vec![];
    }

    let mut variants = vec![cmd.to_string()];
    let Ok(tokens) = shell_words::split(cmd) else {
        return variants;
    };

    for split_at in 1..=tokens.len().min(3) {
        // Safe slicing
        let Some(prefix_tokens) = tokens.get(..split_at) else {
            continue;
        };
        let Some(suffix_tokens) = tokens.get(split_at..) else {
            continue;
        };
        let prefix = prefix_tokens.join(" ");
        let suffix = suffix_tokens.join(" ");
        if suffix.is_empty() {
            variants.push(prefix);
        } else {
            variants.push(format!("{prefix}:{suffix}"));
        }
    }

    variants
}

fn web_fetch_variants(url: &str) -> Vec<String> {
    let url = url.trim();
    if url.is_empty() {
        return vec![];
    }

    let mut out = vec![format!("url:{url}"), url.to_string()];
    match url::Url::parse(url) {
        Ok(parsed) => {
            if let Some(host) = parsed.host_str() {
                out.push(format!("domain:{host}"));
                out.push(host.to_string());
            }
        }
        Err(err) => warn!("Failed to parse web_fetch URL '{}': {}", url, err),
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolConfirmationDecision {
    AllowOnce,
    AllowAlways,
    Deny,
}

fn confirmation_scope(action: &ToolAction, metadata: &ToolMetadata) -> (String, Vec<String>) {
    let permission = action.tool.clone();

    if matches!(
        metadata.category,
        ToolCategory::FileRead
            | ToolCategory::FileWrite
            | ToolCategory::FileSystem
            | ToolCategory::CodeAnalysis
    ) {
        return (permission, vec!["*".to_string()]);
    }

    // For shell (Execution) commands: use the actual command string as patterns.
    // This lets AllowAlways write a specific Bash(prefix:*) rule, while AllowOnce
    // cascade uses permission-level matching (all shell in same task are cascaded).
    let pattern = match action.param_variants.first() {
        Some(pattern) => pattern.clone(),
        None => "*".to_string(),
    };
    (permission, vec![pattern])
}

async fn external_directory_always_patterns(
    metadata: &ToolMetadata,
    context: &AgentRunContext,
    args: &serde_json::Value,
) -> Option<Vec<String>> {
    if !matches!(
        metadata.category,
        ToolCategory::FileRead | ToolCategory::FileWrite | ToolCategory::FileSystem
    ) {
        return None;
    }

    let path = tool_path_arg(args, metadata)?;

    let resolved = match ensure_absolute(&path, &context.cwd) {
        Ok(path) => path,
        Err(err) => {
            warn!(
                "Failed to resolve external-directory approval path '{}': {}",
                path, err
            );
            return None;
        }
    };
    if !resolved.is_absolute() {
        return None;
    }

    let workspace_root = context.session().workspace.clone();
    if !workspace_root.is_absolute() {
        return None;
    }

    if is_within_workspace(&workspace_root, &resolved).await {
        return None;
    }

    let dir = resolved
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolved.clone());
    let canon = match canonicalize_for_workspace_check(&dir).await {
        Some(path) => path,
        None => {
            warn!(
                "Failed to canonicalize external-directory approval path '{}'",
                dir.display()
            );
            return None;
        }
    };
    let canon_str = canon.to_string_lossy();
    Some(vec![format!("{canon_str}/*")])
}

/// Extract a meaningful command prefix for use in a `Bash(prefix:*)` permission rule.
///
/// Strategy (mirrors Claude Code's observable behavior):
///   - If the binary is a well-known subcommand-based CLI (npm, cargo, git, …),
///     take the first **two** tokens so that e.g. `npm run dev` → `npm run`.
///   - Otherwise take only the first token (binary name).
///
/// The result is used as `Bash(<prefix>:*)` in `.orbitx/settings.local.json`.
fn bash_allow_prefix(command: &str) -> String {
    let cmd = command.trim();
    if cmd.is_empty() {
        return String::new();
    }

    // Tools where the second word is a meaningful subcommand worth capturing.
    const SUBCOMMAND_TOOLS: &[&str] = &[
        "npm",
        "npx",
        "yarn",
        "pnpm",
        "bun",
        "cargo",
        "rustup",
        "git",
        "docker",
        "docker-compose",
        "podman",
        "kubectl",
        "helm",
        "terraform",
        "python",
        "python3",
        "pip",
        "pip3",
        "go",
        "mvn",
        "gradle",
        "aws",
        "gcloud",
        "az",
        "make",
    ];

    let Ok(tokens) = shell_words::split(cmd) else {
        // If shell parsing fails, fall back to first whitespace-separated word.
        return cmd.split_whitespace().next().unwrap_or(cmd).to_string();
    };

    let binary = match tokens.first() {
        Some(b) => b.as_str(),
        None => return String::new(),
    };

    if SUBCOMMAND_TOOLS.contains(&binary) {
        if let Some(sub) = tokens.get(1) {
            // Only include the subcommand if it looks like a subcommand (no leading `-`).
            if !sub.starts_with('-') {
                return format!("{binary} {sub}");
            }
        }
    }

    binary.to_string()
}

/// Convert internal `(permission, always_patterns)` from `confirmation_scope` into a settings
/// rule string compatible with `PermissionChecker` / `.orbitx/settings.local.json`.
///
/// Shell: `"npm run dev"` → `"Bash(npm run:*)"` (Claude Code format)
/// Write: path glob    → `"Write(<path>)"`
/// WebFetch: url       → `"WebFetch(domain:<host>)"` or `"WebFetch(url:*)"`
fn permission_to_settings_rule(permission: &str, pattern: &str) -> String {
    match permission {
        "shell" => {
            if pattern == "*" {
                // Fallback: no specific command available.
                "Bash(*)".to_string()
            } else {
                // pattern is the actual command string, e.g. "npm run dev".
                // Extract a meaningful prefix and format as Bash(prefix:*).
                let prefix = bash_allow_prefix(pattern);
                if prefix.is_empty() {
                    "Bash(*)".to_string()
                } else {
                    format!("Bash({prefix}:*)")
                }
            }
        }
        "write" => {
            if pattern == "*" {
                "Write(**)".to_string()
            } else {
                format!("Write({pattern})")
            }
        }
        "edit" | "multi_edit" => {
            if pattern == "*" {
                "Edit(**)".to_string()
            } else {
                format!("Edit({pattern})")
            }
        }
        "read" => {
            if pattern == "*" {
                "Read(**)".to_string()
            } else {
                format!("Read({pattern})")
            }
        }
        "web_fetch" => {
            if pattern == "*" {
                "WebFetch(*)".to_string()
            } else {
                format!("WebFetch({pattern})")
            }
        }
        "web_search" => "WebSearch".to_string(),
        other => {
            if pattern == "*" {
                other.to_string()
            } else {
                format!("{other}({pattern})")
            }
        }
    }
}

/// Check whether a `(permission, always_patterns)` tuple is already allowed by the
/// rules in `.orbitx/settings.local.json`.
async fn is_preapproved_in_local_settings(
    settings_mgr: &crate::settings::SettingsManager,
    workspace_root: &std::path::Path,
    permission: &str,
    always_patterns: &[String],
) -> bool {
    let local = match settings_mgr
        .get_workspace_local_settings(workspace_root)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return false,
        Err(err) => {
            tracing::warn!(
                "Failed to read local settings for pre-approval check: {}",
                err
            );
            return false;
        }
    };

    // Build a temporary PermissionChecker from the local rules so we can use its
    // pattern matching logic rather than reimplementing it here.
    let checker = crate::agent::permissions::PermissionChecker::new(&local.permissions);

    // Build a synthetic ToolAction representing the permission + patterns and ask whether
    // the local settings explicitly allow it.
    always_patterns.iter().all(|p| {
        // Translate back to the rule string that PermissionChecker understands.
        let rule = permission_to_settings_rule(permission, p);
        // Parse the rule into a ToolAction for matching.
        if let Some(parsed) = crate::agent::permissions::pattern::PermissionPattern::parse(&rule) {
            let action = crate::agent::permissions::ToolAction {
                tool: parsed.tool.clone(),
                param_variants: parsed
                    .param
                    .as_deref()
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default(),
                workspace_root: workspace_root.to_path_buf(),
            };
            matches!(
                checker.check(&action),
                crate::agent::permissions::PermissionDecision::Allow
            )
        } else {
            false
        }
    })
}

/// Append new allow-rules to `.orbitx/settings.local.json`.
/// Creates the file if it does not exist yet.
async fn persist_approval_rules_to_local_settings(
    settings_mgr: &crate::settings::SettingsManager,
    workspace_root: &std::path::Path,
    permission: &str,
    patterns: &[String],
) -> Result<(), String> {
    let mut local = settings_mgr
        .get_workspace_local_settings(workspace_root)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    for p in patterns {
        let rule = permission_to_settings_rule(permission, p);
        if !local.permissions.allow.contains(&rule) {
            local.permissions.allow.push(rule);
        }
    }

    settings_mgr
        .update_workspace_local_settings(workspace_root, &local)
        .await
        .map_err(|e| e.to_string())
}

/// After persisting AllowAlways rules, automatically resolve all pending confirmations
/// in the same workspace that are now covered by the updated local settings.
async fn cascade_approvals_from_local_settings(
    settings_mgr: &crate::settings::SettingsManager,
    workspace_root: &std::path::Path,
    pending: &DashMap<String, PendingConfirmation>,
) {
    let workspace_str = workspace_root.to_string_lossy().to_string();

    // Collect candidates first to avoid holding the DashMap borrow while doing async I/O.
    let candidates: Vec<(String, String, Vec<String>)> = pending
        .iter()
        .filter(|entry| entry.value().workspace_path == workspace_str)
        .map(|entry| {
            (
                entry.key().clone(),
                entry.value().permission.clone(),
                entry.value().always_patterns.clone(),
            )
        })
        .collect();

    let mut to_resolve = Vec::new();
    for (id, permission, always_patterns) in candidates {
        if is_preapproved_in_local_settings(
            settings_mgr,
            workspace_root,
            &permission,
            &always_patterns,
        )
        .await
        {
            to_resolve.push(id);
        }
    }

    for id in to_resolve {
        if let Some((_, pending)) = pending.remove(&id) {
            if pending
                .tx
                .send(ToolConfirmationDecision::AllowAlways)
                .is_err()
            {
                tracing::warn!(
                    "Failed to auto-resolve tool confirmation '{}' with AllowAlways",
                    id
                );
            }
        }
    }
}

fn summarize_tool_call(
    tool_name: &str,
    metadata: &ToolMetadata,
    args: &serde_json::Value,
) -> String {
    let summary_value = metadata
        .summary_key_arg
        .and_then(|key| args.get(key))
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else {
                v.to_string()
            }
        });

    let summary = match summary_value {
        Some(v) if !v.trim().is_empty() => format!("{}: {}", tool_name, v.trim()),
        _ => tool_name.to_string(),
    };
    truncate_chars(&summary, 240)
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new(None, None, Arc::new(ToolConfirmationManager::new()))
    }
}

impl Default for ToolConfirmationManager {
    fn default() -> Self {
        Self {
            pending_confirmations: DashMap::new(),
            confirmation_state: tokio::sync::Mutex::new(ConfirmationState::default()),
        }
    }
}

impl ToolConfirmationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lookup_task_id(&self, request_id: &str) -> Option<String> {
        self.pending_confirmations
            .get(request_id)
            .map(|entry| entry.value().run_id.clone())
    }
}

async fn is_within_workspace(workspace_root: &Path, resolved: &Path) -> bool {
    let Some(workspace_canon) = canonicalize_for_workspace_check(workspace_root).await else {
        warn!(
            "Failed to canonicalize workspace root for boundary check: {}",
            workspace_root.display()
        );
        return false;
    };
    let Some(resolved_canon) = canonicalize_for_workspace_check(resolved).await else {
        warn!(
            "Failed to canonicalize target path for boundary check: {}",
            resolved.display()
        );
        return false;
    };
    resolved_canon.starts_with(&workspace_canon)
}

async fn canonicalize_for_workspace_check(path: &Path) -> Option<PathBuf> {
    match tokio::fs::canonicalize(path).await {
        Ok(path) => Some(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Some(normalize_path(path)),
        Err(err) => {
            warn!(
                "Failed to canonicalize path for workspace boundary check (path={}): {}",
                path.display(),
                err
            );
            None
        }
    }
}
