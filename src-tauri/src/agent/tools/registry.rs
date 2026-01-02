use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::{mapref::entry::Entry, DashMap};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use uuid::Uuid;

use super::metadata::{RateLimitConfig, ToolCategory, ToolMetadata};
use super::r#trait::{
    RunnableTool, ToolDescriptionContext, ToolPermission, ToolResult, ToolResultContent,
    ToolResultStatus, ToolSchema,
};
use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::types::TaskEvent;
use crate::storage::repositories::AppPreferences;

/// 根据 chat_mode 获取授予的权限集合
pub fn get_permissions_for_mode(mode: &str) -> Vec<ToolPermission> {
    match mode {
        "chat" => vec![ToolPermission::ReadOnly, ToolPermission::Network],
        _ => vec![
            // Agent 模式:全权限（包含 "agent" 和任何其他值）
            ToolPermission::ReadOnly,
            ToolPermission::FileSystem,
            ToolPermission::SystemCommand,
            ToolPermission::Network,
            ToolPermission::Terminal,
        ],
    }
}

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

    fn check_and_record(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        self.calls
            .retain(|&call_time| now.duration_since(call_time) < window);

        if self.calls.len() >= self.config.max_calls as usize {
            return Err(format!(
                "rate limit exceeded ({} calls / {}s)",
                self.config.max_calls, self.config.window_secs
            ));
        }

        self.calls.push(now);
        Ok(())
    }
}

pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn RunnableTool>>,
    metadata_index: DashMap<String, ToolMetadata>,
    category_index: DashMap<ToolCategory, Vec<String>>,
    rate_limiters: DashMap<String, RateLimiter>,
    aliases: DashMap<String, String>,
    granted_permissions: Vec<ToolPermission>,
    execution_stats: DashMap<String, ToolExecutionStats>,
    pending_confirmations:
        DashMap<String, tokio::sync::oneshot::Sender<ToolConfirmationDecision>>,
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
    pub fn new(granted: Vec<ToolPermission>) -> Self {
        Self {
            tools: DashMap::new(),
            metadata_index: DashMap::new(),
            category_index: DashMap::new(),
            rate_limiters: DashMap::new(),
            aliases: DashMap::new(),
            granted_permissions: granted,
            execution_stats: DashMap::new(),
            pending_confirmations: DashMap::new(),
        }
    }

    pub fn resolve_confirmation(
        &self,
        request_id: &str,
        decision: ToolConfirmationDecision,
    ) -> bool {
        let sender = self.pending_confirmations.remove(request_id).map(|(_, tx)| tx);
        match sender {
            Some(tx) => tx.send(decision).is_ok(),
            None => false,
        }
    }

    pub async fn register(
        &self,
        name: &str,
        tool: Arc<dyn RunnableTool>,
        is_chat_mode: bool, // 新增参数
    ) -> ToolExecutorResult<()> {
        let key = name.to_string();
        let granted = &self.granted_permissions;
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
                    if !tool.check_permissions(granted) {
                        warn!(
                            "工具 {} 缺少所需权限 {:?}",
                            name,
                            tool.required_permissions()
                        );
                        return Ok(());
                    }
                }
            }
        } else {
            // Agent 模式:检查权限（现有逻辑）
            if !tool.check_permissions(granted) {
                warn!(
                    "工具 {} 缺少所需权限 {:?}",
                    name,
                    tool.required_permissions()
                );
            }
        }

        match self.tools.entry(key.clone()) {
            Entry::Occupied(_) => {
                return Err(ToolExecutorError::ConfigurationError(format!(
                    "工具 {} 已经注册",
                    name
                ))
                .into());
            }
            Entry::Vacant(entry) => {
                entry.insert(tool);
            }
        }

        self.metadata_index.insert(key.clone(), metadata.clone());

        self.category_index
            .entry(metadata.category)
            .or_insert_with(Vec::new)
            .push(key.clone());

        if let Some(rate_config) = metadata.rate_limit.clone() {
            self.rate_limiters
                .insert(key.clone(), RateLimiter::new(rate_config));
        }

        self.execution_stats
            .insert(key.clone(), ToolExecutionStats::default());

        Ok(())
    }

    pub async fn unregister(&self, name: &str) -> ToolExecutorResult<()> {
        if self.tools.remove(name).is_none() {
            return Err(ToolExecutorError::ToolNotFound(name.to_string()).into());
        }

        self.aliases.retain(|_, v| v != name);
        self.execution_stats.remove(name);
        self.rate_limiters.remove(name);

        let category = self
            .metadata_index
            .remove(name)
            .map(|(_, meta)| meta.category);

        if let Some(category) = category {
            let mut remove_category = false;

            if let Some(mut list) = self.category_index.get_mut(&category) {
                list.retain(|entry| entry != name);
                remove_category = list.is_empty();
            }

            if remove_category {
                self.category_index.remove(&category);
            }
        }

        Ok(())
    }

    pub async fn add_alias(&self, alias: &str, tool_name: &str) -> ToolExecutorResult<()> {
        if self.resolve_name(tool_name).await.is_none() {
            return Err(ToolExecutorError::ToolNotFound(tool_name.to_string()).into());
        }

        self.aliases
            .insert(alias.to_string(), tool_name.to_string());
        Ok(())
    }

    async fn resolve_name(&self, name: &str) -> Option<String> {
        if self.tools.contains_key(name) {
            return Some(name.to_string());
        }

        self.aliases.get(name).map(|entry| entry.clone())
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn RunnableTool>> {
        let resolved = self.resolve_name(name).await?;
        self.tools
            .get(&resolved)
            .map(|entry| Arc::clone(entry.value()))
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
                        "工具未找到".to_string(),
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
                        "工具未配置元数据".to_string(),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        if let Err(message) = self.check_rate_limit(&resolved).await {
            let detail = Some(format!(
                "category={}, priority={}",
                metadata.category.as_str(),
                metadata.priority.as_str()
            ));
            return self
                .make_error_result(
                    &resolved,
                    message,
                    detail,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        if metadata.requires_confirmation {
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
                error!("工具 {} 超时 {:?}", resolved, timeout);

                ToolResult {
                    content: vec![ToolResultContent::Error(format!(
                        "工具 {} 执行超时 (timeout={:?}, priority={})",
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

    async fn check_rate_limit(&self, tool_name: &str) -> Result<(), String> {
        if let Some(mut limiter) = self.rate_limiters.get_mut(tool_name) {
            limiter.check_and_record()?;
        }
        Ok(())
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
                    "任务已取消，工具执行已中止".to_string(),
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
            Err(message) => {
                return Some(
                    self.make_error_result(
                        tool_name,
                        message,
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
            ToolConfirmationDecision::Deny => {
                Some(
                    self.make_error_result(
                        tool_name,
                        format!("用户拒绝执行工具: {}", tool_name),
                        Some(summary),
                        ToolResultStatus::Cancelled,
                        Some("denied".to_string()),
                        start,
                    )
                    .await,
                )
            }
        }
    }

    async fn request_tool_confirmation(
        &self,
        context: &TaskContext,
        workspace_path: &str,
        tool_name: &str,
        summary: &str,
    ) -> Result<ToolConfirmationDecision, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_confirmations.insert(request_id.clone(), tx);

        let _ = context
            .emit_event(TaskEvent::ToolConfirmationRequested {
                task_id: context.task_id.to_string(),
                request_id: request_id.clone(),
                workspace_path: workspace_path.to_string(),
                tool_name: tool_name.to_string(),
                summary: summary.to_string(),
            })
            .await;

        let decision = tokio::select! {
            res = tokio::time::timeout(Duration::from_secs(600), rx) => {
                match res {
                    Ok(Ok(d)) => Ok(d),
                    Ok(Err(_)) => Err("确认通道已关闭".to_string()),
                    Err(_) => Err("等待用户确认超时".to_string()),
                }
            }
            _ = async {
                while !context.is_aborted() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } => Err("任务已取消，确认已中止".to_string())
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
                        format!("工具未找到: {}", tool_name),
                        None,
                        ToolResultStatus::Error,
                        None,
                        start,
                    )
                    .await;
            }
        };

        let granted = &self.granted_permissions;
        if !tool.check_permissions(granted) {
            return self
                .make_error_result(
                    tool_name,
                    format!(
                        "权限不足: {} 需要权限 {:?}",
                        tool_name,
                        tool.required_permissions()
                    ),
                    None,
                    ToolResultStatus::Error,
                    None,
                    start,
                )
                .await;
        }

        if let Err(e) = tool.validate_arguments(&args) {
            return self
                .make_error_result(
                    tool_name,
                    format!("参数验证失败: {}", e),
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
                    format!("前置钩子失败: {}", e),
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
                    warn!("工具 {} 的 after_run 失败: {}", tool_name, e);
                }

                r
            }
            Err(e) => {
                return self
                    .make_error_result(tool_name, e.to_string(), None, ToolResultStatus::Error, None, start)
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
        error!("工具 {} 执行失败: {}", tool_name, error_message);

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
        if let Some(mut stats) = self.execution_stats.get_mut(tool_name) {
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
        self.tools
            .iter()
            .map(|entry| entry.value().schema())
            .collect()
    }

    /// Get tool schemas with context-aware descriptions
    pub fn get_tool_schemas_with_context(
        &self,
        context: &ToolDescriptionContext,
    ) -> Vec<ToolSchema> {
        self.tools
            .iter()
            .map(|entry| {
                let tool = entry.value();
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
        let mut names: Vec<String> = self.tools.iter().map(|entry| entry.key().clone()).collect();
        names.sort();
        names
    }

    pub async fn get_tool_metadata(&self, name: &str) -> Option<ToolMetadata> {
        if let Some(meta) = self.metadata_index.get(name) {
            return Some(meta.clone());
        }

        if let Some(alias) = self.aliases.get(name) {
            let actual = alias.clone();
            return self.metadata_index.get(&actual).map(|entry| entry.clone());
        }

        None
    }

    pub async fn list_tools_by_category(&self, category: ToolCategory) -> Vec<String> {
        self.category_index
            .get(&category)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }
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

fn summarize_tool_call(tool_name: &str, metadata: &ToolMetadata, args: &serde_json::Value) -> String {
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
        Self::new(vec![])
    }
}
