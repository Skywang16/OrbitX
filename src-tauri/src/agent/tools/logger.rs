/*!
 * 工具执行日志记录（平移至 tools 目录）
 * 提供工具执行的详细日志记录功能，包括：
 * - 执行开始和结束
 * - 参数和结果记录
 * - 错误追踪
 * - 性能统计
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::agent::persistence::prelude::{AgentToolCall, RepositoryManager, ToolCallStatus};
use crate::agent::state::context::TaskContext;
use crate::agent::tools::ToolResult;

/// 工具执行日志记录器
pub struct ToolExecutionLogger {
    /// 数据库仓库
    repositories: Arc<RepositoryManager>,
    /// 内存中的日志缓存
    log_cache: Arc<RwLock<Vec<ToolExecutionLog>>>,
    /// 是否启用详细日志
    verbose: bool,
}

/// 工具执行日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionLog {
    /// 日志ID
    pub id: String,
    /// 任务ID
    pub task_id: String,
    /// 工具调用ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行参数
    pub arguments: JsonValue,
    /// 执行结果
    pub result: Option<JsonValue>,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 执行开始时间
    pub started_at: DateTime<Utc>,
    /// 执行结束时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 额外元数据
    pub metadata: Option<JsonValue>,
}

impl ToolExecutionLogger {
    /// 创建新的日志记录器
    pub fn new(repositories: Arc<RepositoryManager>, verbose: bool) -> Self {
        Self {
            repositories,
            log_cache: Arc::new(RwLock::new(Vec::new())),
            verbose,
        }
    }

    /// 记录工具执行开始
    pub async fn log_start(
        &self,
        context: &TaskContext,
        call_id: &str,
        tool_name: &str,
        arguments: &JsonValue,
    ) -> anyhow::Result<String> {
        let log_id = format!("{}_{}", call_id, Utc::now().timestamp_millis());

        let log_entry = ToolExecutionLog {
            id: log_id.clone(),
            task_id: context.task_id.clone(),
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments: arguments.clone(),
            result: None,
            success: false,
            error: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            metadata: None,
        };

        // 添加到缓存
        {
            let mut cache = self.log_cache.write().await;
            cache.push(log_entry.clone());
        }

        // 写入数据库
        self.persist_log_entry(&log_entry).await?;

        if self.verbose {
            info!(
                "工具执行开始: task_id={}, tool={}, call_id= {}",
                context.task_id, tool_name, call_id
            );
            debug!("工具参数: {}", serde_json::to_string_pretty(arguments)?);
        }

        Ok(log_id)
    }

    /// 记录工具执行成功
    pub async fn log_success(
        &self,
        log_id: &str,
        result: &ToolResult,
        duration_ms: u64,
    ) -> anyhow::Result<()> {
        // 更新缓存中的日志
        {
            let mut cache = self.log_cache.write().await;
            if let Some(log) = cache.iter_mut().find(|l| l.id == log_id) {
                log.result = Some(serde_json::to_value(result)?);
                log.success = true;
                log.completed_at = Some(Utc::now());
                log.duration_ms = Some(duration_ms);
            }
        }

        // 更新数据库
        self.update_log_completion(log_id, true, None, Some(result), duration_ms)
            .await?;

        if self.verbose {
            info!("工具执行成功: log_id={}, 耗时={}ms", log_id, duration_ms);
            debug!("工具结果: {:?}", result);
        }

        Ok(())
    }

    /// 记录工具执行失败
    pub async fn log_failure(
        &self,
        log_id: &str,
        error: &str,
        duration_ms: u64,
    ) -> anyhow::Result<()> {
        // 更新缓存中的日志
        {
            let mut cache = self.log_cache.write().await;
            if let Some(log) = cache.iter_mut().find(|l| l.id == log_id) {
                log.success = false;
                log.error = Some(error.to_string());
                log.completed_at = Some(Utc::now());
                log.duration_ms = Some(duration_ms);
            }
        }

        // 更新数据库
        self.update_log_completion(log_id, false, Some(error), None, duration_ms)
            .await?;

        if self.verbose {
            error!(
                "工具执行失败: log_id={}, 耗时={}ms, 错误={}",
                log_id, duration_ms, error
            );
        }

        Ok(())
    }

    /// 持久化日志条目到数据库
    async fn persist_log_entry(&self, log: &ToolExecutionLog) -> anyhow::Result<()> {
        // 创建agent_tool_calls记录
        let tool_call = AgentToolCall {
            id: None,
            task_id: log.task_id.clone(),
            call_id: log.call_id.clone(),
            tool_name: log.tool_name.clone(),
            arguments_json: serde_json::to_string(&log.arguments)?,
            result_json: None,
            status: ToolCallStatus::Running,
            error_message: None,
            started_at: log.started_at,
            completed_at: None,
        };

        self.repositories
            .agent_tool_calls()
            .create(&tool_call)
            .await?;

        Ok(())
    }

    /// 更新日志完成状态
    async fn update_log_completion(
        &self,
        log_id: &str,
        success: bool,
        error: Option<&str>,
        result: Option<&ToolResult>,
        _duration_ms: u64,
    ) -> anyhow::Result<()> {
        // 从缓存中获取原始日志
        let call_id = {
            let cache = self.log_cache.read().await;
            cache
                .iter()
                .find(|l| l.id == log_id)
                .map(|l| l.call_id.clone())
        };

        if let Some(call_id) = call_id {
            let status = if success {
                ToolCallStatus::Completed
            } else {
                ToolCallStatus::Error
            };

            self.repositories
                .agent_tool_calls()
                .update_status(
                    &call_id,
                    status,
                    result.map(|r| serde_json::to_value(r).unwrap_or(JsonValue::Null)),
                    error.map(|e| e.to_string()),
                )
                .await?;
        }

        Ok(())
    }
}

/// 创建全局日志记录器实例
pub fn create_logger(repositories: Arc<RepositoryManager>) -> Arc<ToolExecutionLogger> {
    Arc::new(ToolExecutionLogger::new(repositories, true))
}
