/*!
 * 工具执行日志记录器 (Stage 2 更新)
 *
 * 通过新的 AgentPersistence 接口将工具执行信息持久化到
 * `tool_executions` 表，同时保留原有的事件输出能力。
 */

use chrono::Utc;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{ error };

use crate::agent::core::context::TaskContext;
use crate::agent::error::AgentResult;
use crate::agent::persistence::{AgentPersistence, ToolExecutionStatus};
use crate::agent::tools::ToolResult;
use crate::storage::DatabaseManager;

pub struct ToolExecutionLogger {
    persistence: Arc<AgentPersistence>,
    verbose: bool,
    #[allow(unused)]
    repositories: Arc<DatabaseManager>,
}

impl ToolExecutionLogger {
    pub fn new(
        repositories: Arc<DatabaseManager>,
        persistence: Arc<AgentPersistence>,
        verbose: bool,
    ) -> Self {
        Self {
            persistence,
            verbose,
            repositories,
        }
    }

    /// 记录工具执行开始
    pub async fn log_start(
        &self,
        context: &TaskContext,
        call_id: &str,
        tool_name: &str,
        arguments: &JsonValue,
    ) -> AgentResult<String> {
        let args_str = serde_json::to_string(arguments)?;
        self.persistence
            .tool_executions()
            .record_execution(
                &context.task_id,
                call_id,
                tool_name,
                &args_str,
                ToolExecutionStatus::Running,
                "[]",
                "[]",
                "[]",
            )
            .await?;

        Ok(call_id.to_string())
    }

    /// 记录工具执行成功
    pub async fn log_success(
        &self,
        log_id: &str,
        result: &ToolResult,
        duration_ms: u64,
    ) -> AgentResult<()> {
        let result_json = serde_json::to_string(result)?;
        self.persistence
            .tool_executions()
            .update_status(
                log_id,
                ToolExecutionStatus::Completed,
                Some(&result_json),
                None,
                Some(Utc::now()),
                Some(duration_ms as i64),
            )
            .await?;

        Ok(())
    }

    /// 记录工具执行失败
    pub async fn log_failure(
        &self,
        log_id: &str,
        error_message: &str,
        duration_ms: u64,
    ) -> AgentResult<()> {
        self.persistence
            .tool_executions()
            .update_status(
                log_id,
                ToolExecutionStatus::Error,
                None,
                Some(error_message),
                Some(Utc::now()),
                Some(duration_ms as i64),
            )
            .await?;

        if self.verbose {
            error!(
                "工具执行失败: log_id={}, 错误={}, 耗时={}ms",
                log_id, error_message, duration_ms
            );
        }
        Ok(())
    }

    /// 记录工具执行取消
    pub async fn log_cancelled(&self, log_id: &str) -> AgentResult<()> {
        self.persistence
            .tool_executions()
            .update_status(
                log_id,
                ToolExecutionStatus::Error,
                None,
                Some("cancelled"),
                Some(Utc::now()),
                None,
            )
            .await?;
        Ok(())
    }
}
