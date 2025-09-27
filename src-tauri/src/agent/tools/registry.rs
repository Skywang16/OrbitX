/*!
 * ToolRegistry - 工具注册表（平移至 tools 目录）
 * 负责：注册、查找、执行工具与统计信息
 */

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use super::error::{ToolExecutorError, ToolExecutorResult};
use super::r#trait::{RunnableTool, ToolPermission, ToolResult, ToolSchema};
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

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let start = std::time::Instant::now();
        let tool = self
            .get_tool(tool_name)
            .await
            .ok_or_else(|| ToolExecutorError::ToolNotFound(tool_name.to_string()))?;

        // 权限检查
        let granted = self.granted_permissions.read().await;
        if !tool.check_permissions(&granted) {
            return Err(ToolExecutorError::PermissionDenied {
                tool_name: tool_name.to_string(),
                required_permission: format!("{:?}", tool.required_permissions()),
            }
            .into());
        }

        // 验证参数 & before hook
        tool.validate_arguments(&args)?;
        tool.before_run(context, &args).await?;

        let result = match tool.run(context, args).await {
            Ok(mut r) => {
                let elapsed = start.elapsed().as_millis() as u64;
                r.execution_time_ms = Some(elapsed);
                self.update_stats(tool_name, true, elapsed).await;
                Ok(r)
            }
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as u64;
                self.update_stats(tool_name, false, elapsed).await;
                error!("工具 {} 执行失败: {}", tool_name, e);
                Err(e)
            }
        };

        if let Ok(ref res) = result {
            tool.after_run(context, res).await?;
        }
        result
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
