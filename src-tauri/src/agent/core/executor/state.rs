/*!
 * 任务状态查询和管理
 */

use std::sync::Arc;

use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{FileContextStatus, TaskExecutor, TaskSummary};
use crate::agent::core::types::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};

impl TaskExecutor {
    /// 获取任务摘要信息
    pub async fn get_task_summary(&self, task_id: &str) -> TaskExecutorResult<TaskSummary> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        let (status, current_iteration, error_count, created_at, updated_at) = ctx
            .batch_read_state(|state| {
                (
                    state.execution.runtime_status,
                    state.execution.record.current_iteration,
                    state.execution.record.error_count,
                    state.execution.record.created_at,
                    state.execution.record.updated_at,
                )
            })
            .await;

        Ok(TaskSummary {
            task_id: task_id.to_string(),
            conversation_id: ctx.conversation_id,
            status: format!("{:?}", status).to_lowercase(),
            current_iteration: current_iteration as i32,
            error_count: error_count as i32,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        })
    }

    /// 列出所有活跃任务
    pub async fn list_active_tasks(&self) -> Vec<TaskSummary> {
        let mut summaries = Vec::new();

        for entry in self.active_tasks().iter() {
            let task_id = entry.key();
            if let Ok(summary) = self.get_task_summary(task_id).await {
                summaries.push(summary);
            }
        }

        summaries
    }

    /// 获取conversation的context
    pub async fn get_conversation_context(&self, conversation_id: i64) -> Option<Arc<TaskContext>> {
        self.conversation_contexts()
            .get(&conversation_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// 获取文件上下文状态
    pub async fn get_file_context_status(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<FileContextStatus> {
        let ctx = self
            .get_conversation_context(conversation_id)
            .await
            .ok_or_else(|| {
                TaskExecutorError::InternalError(format!(
                    "No context found for conversation {}",
                    conversation_id
                ))
            })?;

        let files: Vec<String> = ctx
            .file_tracker()
            .get_active_files()
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?
            .into_iter()
            .map(|entry| entry.file_path)
            .collect();

        Ok(FileContextStatus {
            conversation_id,
            file_count: files.len(),
            files,
        })
    }

    /// 清理已完成的任务（释放内存）
    pub async fn cleanup_completed_tasks(&self) -> usize {
        let mut removed = 0;

        // 收集需要清理的task_id
        let to_remove: Vec<String> = self
            .active_tasks()
            .iter()
            .filter_map(|entry| {
                let status = entry
                    .value()
                    .state
                    .try_read()
                    .ok()
                    .map(|state| state.execution.runtime_status);

                if let Some(status) = status {
                    use crate::agent::core::status::AgentTaskStatus;
                    if matches!(
                        status,
                        AgentTaskStatus::Completed
                            | AgentTaskStatus::Cancelled
                            | AgentTaskStatus::Error
                    ) {
                        return Some(entry.key().clone());
                    }
                }
                None
            })
            .collect();

        // 移除
        for task_id in to_remove {
            self.active_tasks().remove(&task_id);
            removed += 1;
        }

        removed
    }

    /// 获取总任务数统计
    pub fn get_stats(&self) -> TaskExecutorStats {
        TaskExecutorStats {
            active_tasks: self.active_tasks().len(),
            conversation_contexts: self.conversation_contexts().len(),
        }
    }

    /// 列出任务
    pub async fn list_tasks(
        &self,
        conversation_id: Option<i64>,
        status_filter: Option<String>,
    ) -> TaskExecutorResult<Vec<TaskSummary>> {
        let persistence = self.agent_persistence();
        let executions = if let Some(conv_id) = conversation_id {
            persistence
                .agent_executions()
                .list_recent_by_conversation(conv_id, 50)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        } else {
            persistence
                .agent_executions()
                .list_recent(50)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        };

        let mut summaries = Vec::new();
        for execution in executions {
            let status = AgentTaskStatus::from(execution.status);
            if let Some(filter) = &status_filter {
                if status.as_str() != filter {
                    continue;
                }
            }

            summaries.push(TaskSummary {
                task_id: execution.execution_id,
                conversation_id: execution.conversation_id,
                status: status.as_str().to_string(),
                current_iteration: execution.current_iteration as i32,
                error_count: execution.error_count as i32,
                created_at: execution.created_at.to_rfc3339(),
                updated_at: execution.updated_at.to_rfc3339(),
            });
        }

        Ok(summaries)
    }
}

/// 任务执行器统计信息
#[derive(Debug, Clone)]
pub struct TaskExecutorStats {
    pub active_tasks: usize,
    pub conversation_contexts: usize,
}
