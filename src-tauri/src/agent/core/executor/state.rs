/*!
 * Agent run status query and management
 */

use std::sync::Arc;

use chrono::Utc;

use crate::agent::core::executor::{AgentRunExecutor, AgentRunSummary};
use crate::agent::error::{AgentRunError, AgentRunResult};

impl AgentRunExecutor {
    /// Get agent run summary information
    pub async fn get_run_summary(&self, run_id: &str) -> AgentRunResult<AgentRunSummary> {
        let ctx = self
            .active_runs()
            .get(run_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| AgentRunError::TaskNotFound(run_id.to_string()))?;

        let (status, current_iteration, error_count, created_at, updated_at) = ctx
            .batch_read_state(|exec| {
                (
                    exec.runtime_status,
                    exec.current_iteration as i64,
                    exec.error_count as i64,
                    Utc::now(),
                    Utc::now(),
                )
            })
            .await;

        Ok(AgentRunSummary {
            run_id: run_id.to_string(),
            thread_id: ctx.thread_id,
            status: format!("{status:?}").to_lowercase(),
            current_iteration: current_iteration as i32,
            error_count: error_count as i32,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        })
    }

    /// Get total active run count statistics
    pub fn get_stats(&self) -> AgentRunExecutorStats {
        AgentRunExecutorStats {
            active_tasks: self.active_runs().len(),
        }
    }

    /// Agent run list: new design only exposes active runs in memory.
    pub async fn list_runs(
        &self,
        thread_id: Option<i64>,
        status_filter: Option<String>,
    ) -> AgentRunResult<Vec<AgentRunSummary>> {
        let mut summaries = Vec::new();

        for entry in self.active_runs().iter() {
            let ctx = entry.value();
            if let Some(target_session) = thread_id {
                if ctx.thread_id != target_session {
                    continue;
                }
            }

            let summary = self.get_run_summary(entry.key()).await?;
            if let Some(filter) = &status_filter {
                if summary.status != *filter {
                    continue;
                }
            }
            summaries.push(summary);
        }

        Ok(summaries)
    }
}

/// Agent run executor statistics
#[derive(Debug, Clone)]
pub struct AgentRunExecutorStats {
    pub active_tasks: usize,
}
