/*!
 * ToolRegistry - 工具注册表（平移至 tools 目录）
 * 负责：注册、查找、执行工具与统计信息
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::error::{ToolExecutorError, ToolExecutorResult};
use super::r#trait::{RunnableTool, ToolPermission, ToolResult, ToolResultContent, ToolSchema};
use crate::agent::state::context::TaskContext;

/// 工具注册表
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn RunnableTool>>>>,
    aliases: Arc<RwLock<HashMap<String, String>>>,
    granted_permissions: Arc<RwLock<Vec<ToolPermission>>>,
    execution_stats: Arc<RwLock<HashMap<String, ToolExecutionStats>>>,
}

/// 工具执行统计
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
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            granted_permissions: Arc::new(RwLock::new(vec![
                ToolPermission::ReadOnly,
                ToolPermission::FileSystem,
                ToolPermission::SystemCommand,
                ToolPermission::Network,
            ])),
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(
        &self,
        name: &str,
        tool: Arc<dyn RunnableTool>,
    ) -> ToolExecutorResult<()> {
        {
            let tools = self.tools.read().await;
            if tools.contains_key(name) {
                return Err(ToolExecutorError::ConfigurationError(format!(
                    "工具 {} 已经注册",
                    name
                ))
                .into());
            }
        }

        // 权限提示（不阻断）
        let granted = self.granted_permissions.read().await;
        if !tool.check_permissions(&granted) {
            tracing::warn!(
                "工具 {} 需要的权限 {:?} 未被授予",
                name,
                tool.required_permissions()
            );
        }

        {
            let mut tools = self.tools.write().await;
            tools.insert(name.to_string(), tool);
        }
        {
            let mut stats = self.execution_stats.write().await;
            stats.insert(name.to_string(), ToolExecutionStats::default());
        }

        info!("成功注册工具: {}", name);
        Ok(())
    }

    pub async fn unregister(&self, name: &str) -> ToolExecutorResult<()> {
        let removed = {
            let mut tools = self.tools.write().await;
            tools.remove(name)
        };
        if removed.is_some() {
            let mut aliases = self.aliases.write().await;
            aliases.retain(|_, v| v != name);
            Ok(())
        } else {
            Err(ToolExecutorError::ToolNotFound(name.to_string()).into())
        }
    }

    pub async fn add_alias(&self, alias: &str, tool_name: &str) -> ToolExecutorResult<()> {
        // 工具必须已存在
        {
            let tools = self.tools.read().await;
            if !tools.contains_key(tool_name) {
                return Err(ToolExecutorError::ToolNotFound(tool_name.to_string()).into());
            }
        }
        let mut aliases = self.aliases.write().await;
        aliases.insert(alias.to_string(), tool_name.to_string());
        debug!("添加工具别名: {} -> {}", alias, tool_name);
        Ok(())
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn RunnableTool>> {
        {
            let tools = self.tools.read().await;
            if let Some(t) = tools.get(name) {
                return Some(Arc::clone(t));
            }
        }
        {
            let aliases = self.aliases.read().await;
            if let Some(actual) = aliases.get(name) {
                let tools = self.tools.read().await;
                return tools.get(actual).map(Arc::clone);
            }
        }
        None
    }

    /// 执行工具 - 完全同步前端逻辑（带超时控制和统一错误处理）
    /// 对应前端 ModifiableTool.execute()
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolResult {
        let start = std::time::Instant::now();

        // 使用 120 秒超时（对应前端的 120000ms）
        let timeout_result = tokio::time::timeout(
            Duration::from_secs(120),
            self.execute_tool_impl(tool_name, context, args, start),
        )
        .await;

        match timeout_result {
            Ok(result) => result,
            Err(_) => {
                let elapsed = start.elapsed().as_millis() as u64;
                self.update_stats(tool_name, false, elapsed).await;
                error!("工具 {} 执行超时", tool_name);

                ToolResult {
                    content: vec![super::r#trait::ToolResultContent::Error {
                        message: format!("工具 {} 执行超时 (120秒)", tool_name),
                        details: None,
                    }],
                    is_error: true,
                    execution_time_ms: Some(elapsed),
                    ext_info: None,
                }
            }
        }
    }

    /// 工具执行的内部实现
    async fn execute_tool_impl(
        &self,
        tool_name: &str,
        context: &TaskContext,
        args: serde_json::Value,
        start: std::time::Instant,
    ) -> ToolResult {
        // 1. 获取工具
        let tool = match self.get_tool(tool_name).await {
            Some(t) => t,
            None => {
                return self
                    .make_error_result(tool_name, format!("工具未找到: {}", tool_name), start)
                    .await;
            }
        };

        // 2. 权限检查
        let granted = self.granted_permissions.read().await;
        if !tool.check_permissions(&granted) {
            return self
                .make_error_result(
                    tool_name,
                    format!(
                        "权限不足: {} 需要权限 {:?}",
                        tool_name,
                        tool.required_permissions()
                    ),
                    start,
                )
                .await;
        }

        // 3. 验证参数
        if let Err(e) = tool.validate_arguments(&args) {
            return self
                .make_error_result(tool_name, format!("参数验证失败: {}", e), start)
                .await;
        }

        // 4. 执行前钩子
        if let Err(e) = tool.before_run(context, &args).await {
            return self
                .make_error_result(tool_name, format!("前置钩子失败: {}", e), start)
                .await;
        }

        // 5. 执行工具
        let result = match tool.run(context, args).await {
            Ok(mut r) => {
                let elapsed = start.elapsed().as_millis() as u64;
                r.execution_time_ms = Some(elapsed);
                self.update_stats(tool_name, true, elapsed).await;

                // 6. 执行后钩子
                if let Err(e) = tool.after_run(context, &r).await {
                    warn!("工具 {} 的 after_run 钩子失败: {}", tool_name, e);
                }

                r
            }
            Err(e) => {
                return self
                    .make_error_result(tool_name, e.to_string(), start)
                    .await;
            }
        };

        result
    }

    /// 创建错误结果（统一错误处理）
    async fn make_error_result(
        &self,
        tool_name: &str,
        error_message: String,
        start: std::time::Instant,
    ) -> ToolResult {
        let elapsed = start.elapsed().as_millis() as u64;
        self.update_stats(tool_name, false, elapsed).await;
        error!("工具 {} 执行失败: {}", tool_name, error_message);

        ToolResult {
            content: vec![ToolResultContent::Error {
                message: error_message,
                details: None,
            }],
            is_error: true,
            execution_time_ms: Some(elapsed),
            ext_info: None,
        }
    }

    async fn update_stats(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        let mut stats = self.execution_stats.write().await;
        if let Some(s) = stats.get_mut(tool_name) {
            s.total_calls += 1;
            if success {
                s.success_count += 1;
            } else {
                s.failure_count += 1;
            }
            s.total_execution_time_ms += execution_time_ms;
            s.avg_execution_time_ms = s.total_execution_time_ms / s.total_calls.max(1);
            s.last_called_at = Some(chrono::Utc::now());
        }
    }

    pub async fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.schema()).collect()
    }

    pub async fn list_tools(&self) -> Vec<String> {
        let tools = self.tools.read().await;
        let mut names: Vec<String> = tools.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
