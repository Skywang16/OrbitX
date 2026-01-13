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
use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::types::TaskEvent;
use crate::agent::{
    permissions::PermissionChecker, permissions::PermissionDecision, permissions::ToolAction,
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
    permission_checker: Option<Arc<PermissionChecker>>,
    pending_confirmations: DashMap<String, tokio::sync::oneshot::Sender<ToolConfirmationDecision>>,
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
    pub fn new(permission_checker: Option<Arc<PermissionChecker>>) -> Self {
        Self {
            aliases: DashMap::new(),
            entries: DashMap::new(),
            permission_checker,
            pending_confirmations: DashMap::new(),
        }
    }

    pub fn resolve_confirmation(
        &self,
        request_id: &str,
        decision: ToolConfirmationDecision,
    ) -> bool {
        let sender = self
            .pending_confirmations
            .remove(request_id)
            .map(|(_, tx)| tx);
        match sender {
            Some(tx) => tx.send(decision).is_ok(),
            None => false,
        }
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
                    "Tool already registered: {}",
                    name
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

        let permission_decision = self.permission_checker.as_ref().map(|checker| {
            let action = build_tool_action(&resolved, &metadata, context, &args);
            (checker.check(&action), action)
        });

        if let Some((PermissionDecision::Deny, action)) = &permission_decision {
            return self
                .make_error_result(
                    &resolved,
                    format!("Denied by settings permissions: {}", resolved),
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

        let requires_confirmation = match permission_decision {
            Some((PermissionDecision::Allow, _)) => false,
            Some((PermissionDecision::Ask, _)) => true,
            Some((PermissionDecision::Deny, _)) => true, // handled above
            None => {
                metadata.requires_confirmation
                    || self
                        .requires_workspace_confirmation(&metadata, context, &args)
                        .await
            }
        };

        if requires_confirmation {
            if let Some(blocked) = self
                .confirm_or_block_tool(&resolved, &metadata, context, &args, start)
                .await
            {
                return blocked;
            }
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
        if !matches!(
            metadata.category,
            ToolCategory::FileRead
                | ToolCategory::FileWrite
                | ToolCategory::FileSystem
                | ToolCategory::CodeAnalysis
        ) {
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
        let preference_key = confirmation_preference_key(&workspace, tool_name);

        let db = context.session().repositories();
        let stored = AppPreferences::new(db.as_ref())
            .get(&preference_key)
            .await
            .ok()
            .flatten();
        if matches!(stored.as_deref(), Some("allow")) {
            return None;
        }

        let summary = summarize_tool_call(tool_name, metadata, args);
        let decision = match self
            .request_tool_confirmation(context, &workspace, tool_name, &summary)
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
            ToolConfirmationDecision::AllowAlways => {
                let _ = AppPreferences::new(db.as_ref())
                    .set(&preference_key, Some("allow"))
                    .await;
                None
            }
            ToolConfirmationDecision::Deny => Some(
                self.make_error_result(
                    tool_name,
                    format!("User denied tool execution: {}", tool_name),
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
    ) -> ToolExecutorResult<ToolConfirmationDecision> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_confirmations.insert(request_id.clone(), tx);

        if let Err(err) = context
            .emit_event(TaskEvent::ToolConfirmationRequested {
                task_id: context.task_id.to_string(),
                request_id: request_id.clone(),
                workspace_path: workspace_path.to_string(),
                tool_name: tool_name.to_string(),
                summary: summary.to_string(),
            })
            .await
        {
            self.pending_confirmations.remove(&request_id);
            return Err(ToolExecutorError::ExecutionFailed {
                tool_name: tool_name.to_string(),
                error: format!(
                    "Failed to request user confirmation (UI channel unavailable): {}",
                    err
                ),
            });
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
            self.pending_confirmations.remove(&request_id);
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
                return self
                    .make_error_result(
                        tool_name,
                        format!("Tool not found: {}", tool_name),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        if let Err(e) = tool.validate_arguments(&args) {
            return self
                .make_error_result(
                    tool_name,
                    format!("Argument validation failed: {}", e),
                    None,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        if let Err(e) = tool.before_run(context, &args).await {
            return self
                .make_error_result(
                    tool_name,
                    format!("Pre-run hook failed: {}", e),
                    None,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        let result = match tool.run(context, args).await {
            Ok(mut r) => {
                let elapsed = start.elapsed().as_millis() as u64;
                r.execution_time_ms = Some(elapsed);
                self.update_stats(tool_name, true, elapsed).await;

                if let Err(e) = tool.after_run(context, &r).await {
                    warn!("Tool {} after_run hook failed: {}", tool_name, e);
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
        };

        result
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
            format!("{} ({})", error_message, d)
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
        self.entries
            .iter()
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
            ToolAction::new("Bash", workspace_root, bash_param_variants(command))
        }
        "read_file" => ToolAction::new(
            "Read",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "write_file" => ToolAction::new(
            "Write",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "edit_file" => ToolAction::new(
            "Edit",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "list_files" | "grep" | "semantic_search" => ToolAction::new(
            "Read",
            workspace_root,
            path_variants(args, metadata, context),
        ),
        "web_fetch" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            ToolAction::new("WebFetch", workspace_root, web_fetch_variants(url))
        }
        "read_terminal" => ToolAction::new("Terminal", workspace_root, vec![]),
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

fn confirmation_preference_key(workspace_path: &str, tool_name: &str) -> String {
    let digest = blake3::hash(workspace_path.as_bytes());
    format!("agent.tool_confirmation.{}/{}", digest.to_hex(), tool_name)
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

    let mut summary = match summary_value {
        Some(v) if !v.trim().is_empty() => format!("{}: {}", tool_name, v.trim()),
        _ => tool_name.to_string(),
    };

    const MAX_LEN: usize = 240;
    if summary.len() > MAX_LEN {
        summary.truncate(MAX_LEN);
        summary.push_str("…");
    }
    summary
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new(None)
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
