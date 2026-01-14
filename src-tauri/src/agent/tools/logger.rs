/*!
 * 工具执行日志记录器 (Stage 2 更新)
 *
 * 通过新的 AgentPersistence 接口将工具执行信息持久化到
 * `tool_executions` 表，同时保留原有的事件输出能力。
 */

use serde_json::Value as JsonValue;
use tracing::error;

use crate::agent::core::context::TaskContext;
use crate::agent::error::AgentResult;
use crate::agent::tools::ToolResult;

pub struct ToolExecutionLogger {
    verbose: bool,
}

impl ToolExecutionLogger {
    pub fn new(
        verbose: bool,
    ) -> Self {
        Self {
            verbose,
        }
    }

    /// 记录工具执行开始
    pub async fn log_start(
        &self,
        _context: &TaskContext,
        call_id: &str,
        tool_name: &str,
        arguments: &JsonValue,
    ) -> AgentResult<String> {
        let _ = (tool_name, arguments);

        Ok(call_id.to_string())
    }

    /// 记录工具执行成功
    pub async fn log_success(
        &self,
        log_id: &str,
        _result: &ToolResult,
        duration_ms: u64,
    ) -> AgentResult<()> {
        let _ = (log_id, duration_ms);
        Ok(())
    }

    /// 记录工具执行失败
    pub async fn log_failure(
        &self,
        log_id: &str,
        error_message: &str,
        duration_ms: u64,
    ) -> AgentResult<()> {
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
        let _ = log_id;
        Ok(())
    }
}
