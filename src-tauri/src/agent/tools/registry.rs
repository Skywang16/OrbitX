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
use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::types::TaskEvent;
use crate::agent::{
    permissions::PermissionChecker, permissions::PermissionDecision, permissions::ToolAction,
    permissions::ToolFilter,
};
use crate::storage::repositories::AppPreferences;

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
/// Multiple ToolRegistry instances exist in multi-agent/subtask mode; the UI only supports one
/// confirmation dialog at a time, so confirmation state must be shared to avoid deadlocks.
pub struct ToolConfirmationManager {
    pending_confirmations: DashMap<String, PendingConfirmation>,
    confirmation_state: tokio::sync::Mutex<ConfirmationState>,
}

struct PendingConfirmation {
    tx: tokio::sync::oneshot::Sender<ToolConfirmationDecision>,
    task_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredApprovalRule {
    permission: String,
    pattern: String,
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
    /// 唯一的构造函数 - 显式传递权限
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
        context: &TaskContext,
        request_id: &str,
        decision: ToolConfirmationDecision,
    ) -> bool {
        let removed = self.confirmations.pending_confirmations.remove(request_id);
        let Some((_, pending)) = removed else {
            return false;
        };

        let workspace = pending.workspace_path.clone();
        let task_id = pending.task_id.clone();
        let permission = pending.permission.clone();
        let always_patterns = pending.always_patterns.clone();

        let ok = pending.tx.send(decision).is_ok();

        match decision {
            ToolConfirmationDecision::AllowAlways => {
                let db = context.session().repositories();
                let _ =
                    persist_approval_rules(db.as_ref(), &workspace, &permission, &always_patterns)
                        .await;
                cascade_approvals(db.as_ref(), &workspace, &self.confirmations.pending_confirmations)
                    .await;
            }
            ToolConfirmationDecision::AllowOnce => {
                self.cascade_allow_once(&task_id, &workspace, &permission, &always_patterns);
            }
            ToolConfirmationDecision::Deny => {
                self.cancel_pending_confirmations_for_task(context, &task_id).await;
            }
        }

        self.finish_confirmation_and_pump_next(context, request_id)
            .await;

        ok
    }

    pub async fn cancel_pending_confirmations_for_task(&self, context: &TaskContext, task_id: &str) {
        let to_cancel = self
            .confirmations
            .pending_confirmations
            .iter()
            .filter(|entry| entry.value().task_id == task_id)
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
        task_id: &str,
        workspace_path: &str,
        permission: &str,
        always_patterns: &[String],
    ) {
        let mut to_resolve = Vec::new();
        for entry in self.confirmations.pending_confirmations.iter() {
            let id = entry.key().clone();
            let p = entry.value();
            if p.task_id != task_id {
                continue;
            }
            if p.workspace_path != workspace_path {
                continue;
            }
            if p.permission != permission {
                continue;
            }
            if p.always_patterns != always_patterns {
                continue;
            }
            to_resolve.push(id);
        }

        for id in to_resolve {
            if let Some((_, pending)) = self.confirmations.pending_confirmations.remove(&id) {
                let _ = pending.tx.send(ToolConfirmationDecision::AllowOnce);
            }
        }
    }

    async fn finish_confirmation_and_pump_next(&self, context: &TaskContext, request_id: &str) {
        let mut state = self.confirmations.confirmation_state.lock().await;

        if state.active_request_id.as_deref() == Some(request_id) {
            state.active_request_id = None;
        } else {
            // If it was queued (shouldn't happen with a single-dialog UI), drop it.
            state.queue.retain(|id| id != request_id);
        }

        drop(state);
        self.pump_next_confirmation(context).await;
    }

    async fn pump_next_confirmation(&self, context: &TaskContext) {
        let mut state = self.confirmations.confirmation_state.lock().await;

        // Only pump when there is no active request.
        if state.active_request_id.is_some() {
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
        };

        let Some(next_id) = next else {
            return;
        };

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
        if let Some((_, pending)) = self
            .confirmations
            .pending_confirmations
            .remove(request_id)
        {
            let _ = pending.tx.send(decision);
        }
        let mut state = self.confirmations.confirmation_state.lock().await;
        if state.active_request_id.as_deref() == Some(request_id) {
            state.active_request_id = None;
        }
        state.queue.retain(|id| id != request_id);
    }

    async fn emit_confirmation_request(
        &self,
        context: &TaskContext,
        request_id: &str,
    ) -> ToolExecutorResult<()> {
        let pending = self
            .confirmations
            .pending_confirmations
            .get(request_id)
            .ok_or_else(|| {
            ToolExecutorError::ExecutionFailed {
                tool_name: "tool_confirmation".to_string(),
                error: "Pending confirmation not found".to_string(),
            }
        })?;

        context
            .emit_event(TaskEvent::ToolConfirmationRequested {
                task_id: pending.task_id.clone(),
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

        // === Chat 模式工具过滤逻辑 ===
        if is_chat_mode {
            // 黑名单:禁止 FileWrite 和 Execution 类别
            match metadata.category {
                ToolCategory::FileWrite | ToolCategory::Execution => {
                    return Ok(()); // 静默跳过,不注册
                }
                // 白名单:允许只读类工具
                ToolCategory::FileRead | ToolCategory::CodeAnalysis | ToolCategory::FileSystem => {
                    // 直接允许,无需权限检查
                }
                // 其他类别:检查权限
                _ => {
                    // permissions are enforced at runtime via settings.json (allow/deny/ask)
                }
            }
        } else {
            // Agent 模式: permissions are enforced at runtime via settings.json (allow/deny/ask)
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

    pub async fn unregister(&self, name: &str) -> ToolExecutorResult<()> {
        if self.entries.remove(name).is_none() {
            return Err(ToolExecutorError::ToolNotFound(name.to_string()));
        }

        self.aliases.retain(|_, v| v != name);

        Ok(())
    }

    pub async fn add_alias(&self, alias: &str, tool_name: &str) -> ToolExecutorResult<()> {
        if self.resolve_name(tool_name).await.is_none() {
            return Err(ToolExecutorError::ToolNotFound(tool_name.to_string()));
        }

        self.aliases
            .insert(alias.to_string(), tool_name.to_string());
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
        context: &TaskContext,
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
        let (settings_decision, settings_matched) = self
            .settings_permissions
            .as_ref()
            .map(|checker| {
                let (decision, matched) = checker.check_with_match(&action);
                (Some(decision), matched)
            })
            .unwrap_or((None, false));
        // Good taste: "no matching rule" is not the same thing as "Ask".
        // If the user's allow/deny/ask lists don't match this action, treat it as "no decision"
        // and let tool metadata + workspace boundary checks drive whether we prompt.
        let settings_decision = if settings_matched { settings_decision } else { None };

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
            // `task` is orchestration, not a side-effecting tool. It should never be blocked by
            // confirmation prompts (only by explicit deny rules).
            _ if resolved == "task" && !matches!(settings_decision, Some(PermissionDecision::Deny)) => false,
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
            tracing::info!("⏸️  Waiting for confirmation: {}", resolved);
            if let Some(blocked) = self
                .confirm_or_block_tool(&resolved, &metadata, context, &args, &action, start)
                .await
            {
                tracing::info!("🚫 Tool denied: {}", resolved);
                return blocked;
            }
            tracing::info!("✅ Tool confirmed: {}", resolved);
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

    fn effective_permission_decision(&self, tool_name: &str, action: &ToolAction) -> PermissionDecision {
        // Agent tool filter: if tool is not allowed, treat as Deny
        if self
            .agent_tool_filter
            .as_ref()
            .is_some_and(|filter| !filter.is_allowed(tool_name))
        {
            return PermissionDecision::Deny;
        }

        self.settings_permissions
            .as_ref()
            .map(|checker| checker.check(action))
            .unwrap_or(PermissionDecision::Allow)
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
        context: &TaskContext,
        args: &serde_json::Value,
    ) -> bool {
        // Only write operations need workspace boundary confirmation.
        // Read-only tools (FileRead, FileSystem, CodeAnalysis) should never require confirmation.
        if !matches!(metadata.category, ToolCategory::FileWrite) {
            return false;
        }

        let path = args.get("path").and_then(|v| v.as_str()).or_else(|| {
            metadata
                .summary_key_arg
                .and_then(|key| args.get(key))
                .and_then(|v| v.as_str())
        });

        let Some(path) = path else {
            return false;
        };

        let resolved_path =
            match crate::agent::tools::builtin::file_utils::ensure_absolute(path, &context.cwd) {
                Ok(p) => p,
                Err(_) => return false,
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
        context: &TaskContext,
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

        let db = context.session().repositories();

        if let Some(ext) = external_directory_always_patterns(metadata, context, args).await {
            if !is_preapproved(db.as_ref(), &workspace, "external_directory", &ext).await {
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

        if is_preapproved(db.as_ref(), &workspace, &permission, &always_patterns).await {
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
        context: &TaskContext,
        workspace_path: &str,
        tool_name: &str,
        summary: &str,
        permission: &str,
        always_patterns: &[String],
    ) -> ToolExecutorResult<ToolConfirmationDecision> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.confirmations.pending_confirmations.insert(
            request_id.clone(),
            PendingConfirmation {
                tx,
                task_id: context.task_id.to_string(),
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
                true
            } else {
                state.queue.push_back(request_id.clone());
                false
            }
        };

        if should_emit {
            if let Err(err) = self.emit_confirmation_request(context, &request_id).await {
                self.confirmations.pending_confirmations.remove(&request_id);
                self.finish_confirmation_and_pump_next(context, &request_id)
                    .await;
                return Err(err);
            }
        }

        let decision = tokio::select! {
            res = tokio::time::timeout(Duration::from_secs(600), rx) => {
                match res {
                    Ok(Ok(d)) => Ok(d),
                    Ok(Err(_)) => Err(ToolExecutorError::ExecutionFailed {
                        tool_name: tool_name.to_string(),
                        error: "Confirmation channel closed".to_string(),
                    }),
                    Err(_) => Err(ToolExecutorError::ExecutionTimeout {
                        tool_name: tool_name.to_string(),
                        timeout_seconds: 600,
                    }),
                }
            }
            _ = context.states.abort_token.cancelled() => Err(ToolExecutorError::ExecutionFailed {
                tool_name: tool_name.to_string(),
                error: "Task aborted; confirmation cancelled".to_string(),
            })
        };

        if decision.is_err() {
            self.confirmations.pending_confirmations.remove(&request_id);
            self.finish_confirmation_and_pump_next(context, &request_id)
                .await;
        }

        decision
    }

    async fn execute_tool_impl(
        &self,
        tool_name: &str,
        context: &TaskContext,
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
                    tracing::info!("🔧 {} completed in {:.1}s", tool_name, elapsed as f64 / 1000.0);
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

    pub async fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        self.entries
            .iter()
            .map(|entry| entry.value().tool.schema())
            .collect()
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
                if self.effective_permission_decision(tool_name, &action) == PermissionDecision::Deny {
                    return false;
                }

                true
            })
            .map(|entry| {
                let tool = &entry.value().tool;
                let description = tool
                    .description_with_context(context)
                    .unwrap_or_else(|| tool.description().to_string());

                ToolSchema {
                    name: tool.name().to_string(),
                    description,
                    parameters: tool.parameters_schema(),
                }
            })
            .collect()
    }

    pub async fn list_tools(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .entries
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        names.sort();
        names
    }

    pub async fn list_tools_by_category(&self, category: ToolCategory) -> Vec<String> {
        let mut out: Vec<String> = self
            .entries
            .iter()
            .filter_map(|entry| {
                if entry.value().metadata.category == category {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
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
        "list_files" => ToolAction::new("list", workspace_root, vec![]),
        "grep" => ToolAction::new("grep", workspace_root, vec![]),
        "semantic_search" => ToolAction::new("semantic_search", workspace_root, vec![]),
        "read_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "read_agent_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "syntax_diagnostics" => ToolAction::new("syntax_diagnostics", workspace_root, vec![]),
        "todowrite" => ToolAction::new("todowrite", workspace_root, vec![]),
        "task" => ToolAction::new("task", workspace_root, vec![]),
        _ => ToolAction::new(tool_name, workspace_root, vec![]),
    }
}

fn build_tool_action(
    tool_name: &str,
    metadata: &ToolMetadata,
    context: &TaskContext,
    args: &serde_json::Value,
) -> ToolAction {
    let workspace_root = PathBuf::from(context.cwd.as_ref());

    if tool_name.starts_with("mcp__") {
        return ToolAction::new(tool_name, workspace_root, vec![]);
    }

    match tool_name {
        "shell" => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            ToolAction::new("shell", workspace_root, bash_param_variants(command))
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
        "list_files" => ToolAction::new(
            "list",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "grep" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            ToolAction::new("grep", workspace_root, vec![query.to_string()])
        }
        "semantic_search" => ToolAction::new("semantic_search", workspace_root, vec![]),
        "web_fetch" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            ToolAction::new("web_fetch", workspace_root, web_fetch_variants(url))
        }
        "web_search" => {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or_default();
            ToolAction::new("web_search", workspace_root, vec![q.to_string()])
        }
        "read_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "read_agent_terminal" => ToolAction::new("terminal", workspace_root, vec![]),
        "syntax_diagnostics" => ToolAction::new("syntax_diagnostics", workspace_root, vec![]),
        "todowrite" => ToolAction::new("todowrite", workspace_root, vec![]),
        "task" => {
            let agent = args
                .get("subagent_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let variants = if agent.trim().is_empty() {
                vec![]
            } else {
                vec![agent.trim().to_string()]
            };
            ToolAction::new("task", workspace_root, variants)
        }
        _ => {
            let summary = metadata
                .summary_key_arg
                .and_then(|key| args.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let variants = if summary.is_empty() {
                vec![]
            } else {
                vec![summary]
            };
            ToolAction::new(tool_name, workspace_root, variants)
        }
    }
}

fn path_variants(
    args: &serde_json::Value,
    metadata: &ToolMetadata,
    context: &TaskContext,
) -> Vec<String> {
    let path = args.get("path").and_then(|v| v.as_str()).or_else(|| {
        metadata
            .summary_key_arg
            .and_then(|key| args.get(key))
            .and_then(|v| v.as_str())
    });

    let Some(path) = path else {
        return vec![context.cwd.to_string()];
    };

    match crate::agent::tools::builtin::file_utils::ensure_absolute(path, &context.cwd) {
        Ok(resolved) => vec![resolved.to_string_lossy().to_string()],
        Err(_) => vec![path.to_string()],
    }
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
        // 安全切片
        let prefix = tokens
            .get(..split_at)
            .map(|t| t.join(" "))
            .unwrap_or_default();
        let suffix = tokens
            .get(split_at..)
            .map(|t| t.join(" "))
            .unwrap_or_default();
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
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            out.push(format!("domain:{host}"));
            out.push(host.to_string());
        }
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

fn approval_rules_preference_key(workspace_path: &str) -> String {
    let digest = blake3::hash(workspace_path.as_bytes());
    format!("agent.tool_confirmation.ruleset.{}", digest.to_hex())
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

    let pattern = action
        .param_variants
        .first()
        .cloned()
        .unwrap_or_else(|| "*".to_string());
    (permission, vec![pattern])
}

async fn external_directory_always_patterns(
    metadata: &ToolMetadata,
    context: &TaskContext,
    args: &serde_json::Value,
) -> Option<Vec<String>> {
    if !matches!(
        metadata.category,
        ToolCategory::FileRead | ToolCategory::FileWrite | ToolCategory::FileSystem
    ) {
        return None;
    }

    let path = args.get("path").and_then(|v| v.as_str()).or_else(|| {
        metadata
            .summary_key_arg
            .and_then(|key| args.get(key))
            .and_then(|v| v.as_str())
    })?;

    let resolved = crate::agent::tools::builtin::file_utils::ensure_absolute(path, &context.cwd)
        .ok()
        .unwrap_or_else(|| PathBuf::from(path));
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

    let dir = resolved.parent().unwrap_or(&resolved).to_path_buf();
    let canon = tokio::fs::canonicalize(&dir).await.unwrap_or(dir);
    let canon_str = canon.to_string_lossy();
    Some(vec![format!("{canon_str}/*")])
}

async fn load_approval_rules(
    db: &crate::storage::DatabaseManager,
    workspace_path: &str,
) -> std::collections::HashSet<(String, String)> {
    let key = approval_rules_preference_key(workspace_path);
    let stored = AppPreferences::new(db)
        .get(&key)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if stored.trim().is_empty() {
        return std::collections::HashSet::new();
    }
    let Ok(list) = serde_json::from_str::<Vec<StoredApprovalRule>>(&stored) else {
        return std::collections::HashSet::new();
    };
    list.into_iter()
        .map(|r| (r.permission, r.pattern))
        .collect()
}

async fn is_preapproved(
    db: &crate::storage::DatabaseManager,
    workspace_path: &str,
    permission: &str,
    always_patterns: &[String],
) -> bool {
    let rules = load_approval_rules(db, workspace_path).await;
    always_patterns
        .iter()
        .all(|p| rules.contains(&(permission.to_string(), p.clone())))
}

async fn persist_approval_rules(
    db: &crate::storage::DatabaseManager,
    workspace_path: &str,
    permission: &str,
    patterns: &[String],
) -> ToolExecutorResult<()> {
    let key = approval_rules_preference_key(workspace_path);
    let prefs = AppPreferences::new(db);

    let existing = prefs.get(&key).await.ok().flatten().unwrap_or_default();
    let mut rules = if existing.trim().is_empty() {
        Vec::<StoredApprovalRule>::new()
    } else {
        serde_json::from_str::<Vec<StoredApprovalRule>>(&existing).unwrap_or_default()
    };

    for p in patterns {
        if rules
            .iter()
            .any(|r| r.permission == permission && r.pattern == *p)
        {
            continue;
        }
        rules.push(StoredApprovalRule {
            permission: permission.to_string(),
            pattern: p.clone(),
        });
    }

    let json = serde_json::to_string(&rules).unwrap_or_default();
    let _ = prefs.set(&key, Some(json.as_str())).await;
    Ok(())
}

async fn cascade_approvals(
    db: &crate::storage::DatabaseManager,
    workspace_path: &str,
    pending: &DashMap<String, PendingConfirmation>,
) {
    let rules = load_approval_rules(db, workspace_path).await;

    let mut to_resolve = Vec::new();
    for entry in pending.iter() {
        let id = entry.key().clone();
        let p = entry.value();
        if p.workspace_path != workspace_path {
            continue;
        }
        let ok = p
            .always_patterns
            .iter()
            .all(|pat| rules.contains(&(p.permission.clone(), pat.clone())));
        if ok {
            to_resolve.push(id);
        }
    }

    for id in to_resolve {
        if let Some((_, pending)) = pending.remove(&id) {
            let _ = pending.tx.send(ToolConfirmationDecision::AllowAlways);
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

impl ToolConfirmationManager {
    pub fn new() -> Self {
        Self {
            pending_confirmations: DashMap::new(),
            confirmation_state: tokio::sync::Mutex::new(ConfirmationState::default()),
        }
    }

    pub fn lookup_task_id(&self, request_id: &str) -> Option<String> {
        self.pending_confirmations
            .get(request_id)
            .map(|entry| entry.value().task_id.clone())
    }
}

async fn is_within_workspace(workspace_root: &Path, resolved: &Path) -> bool {
    let workspace_canon = tokio::fs::canonicalize(workspace_root)
        .await
        .unwrap_or_else(|_| workspace_root.to_path_buf());

    let resolved_canon = tokio::fs::canonicalize(resolved)
        .await
        .unwrap_or_else(|_| resolved.to_path_buf());

    resolved_canon.starts_with(&workspace_canon)
}
