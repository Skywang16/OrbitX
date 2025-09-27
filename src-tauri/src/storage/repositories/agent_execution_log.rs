/*!
 * Agent执行日志Repository实现
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::storage::repositories::agent::{AgentExecutionLog, ExecutionStepType};
use crate::utils::error::AppResult;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Agent执行日志Repository
pub struct AgentExecutionLogRepository {
    database: Arc<DatabaseManager>,
}

impl AgentExecutionLogRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 创建执行日志
    pub async fn create(&self, log: &AgentExecutionLog) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("agent_execution_log")
            .set("task_id", Value::String(log.task_id.clone()))
            .set("iteration", Value::Number(log.iteration.into()))
            .set(
                "step_type",
                Value::String(log.step_type.as_str().to_string()),
            )
            .set("content_json", Value::String(log.content_json.clone()))
            .set("timestamp", Value::String(log.timestamp.to_rfc3339()))
            .build()?;

        let mut query = sqlx::query(&sql);
        for param in params {
            query = match param {
                Value::String(s) => query.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query.bind(f)
                    } else {
                        query.bind(0i64)
                    }
                }
                _ => query.bind(param.to_string()),
            };
        }

        let result = query.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    /// 根据任务ID查找执行日志
    pub async fn find_by_task_id(&self, task_id: &str) -> AppResult<Vec<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log WHERE task_id = ? ORDER BY iteration ASC, timestamp ASC";
        let rows = sqlx::query(sql)
            .bind(task_id)
            .fetch_all(self.database.pool())
            .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(AgentExecutionLog::from_row(&row)?);
        }

        Ok(logs)
    }

    /// 根据任务ID和迭代号查找执行日志
    pub async fn find_by_task_and_iteration(
        &self,
        task_id: &str,
        iteration: u32,
    ) -> AppResult<Vec<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log WHERE task_id = ? AND iteration = ? ORDER BY timestamp ASC";
        let rows = sqlx::query(sql)
            .bind(task_id)
            .bind(iteration as i64)
            .fetch_all(self.database.pool())
            .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(AgentExecutionLog::from_row(&row)?);
        }

        Ok(logs)
    }

    /// 根据步骤类型查找执行日志
    pub async fn find_by_step_type(
        &self,
        task_id: &str,
        step_type: ExecutionStepType,
    ) -> AppResult<Vec<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log WHERE task_id = ? AND step_type = ? ORDER BY timestamp ASC";
        let rows = sqlx::query(sql)
            .bind(task_id)
            .bind(step_type.as_str())
            .fetch_all(self.database.pool())
            .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(AgentExecutionLog::from_row(&row)?);
        }

        Ok(logs)
    }

    /// 获取最新的执行日志
    pub async fn get_latest_by_task(&self, task_id: &str) -> AppResult<Option<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log WHERE task_id = ? ORDER BY iteration DESC, timestamp DESC LIMIT 1";
        let row = sqlx::query(sql)
            .bind(task_id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentExecutionLog::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 清理过期的执行日志
    pub async fn cleanup_old_logs(&self, days: i32) -> AppResult<u64> {
        let sql = "DELETE FROM agent_execution_log WHERE timestamp < datetime('now', ?) AND task_id IN (SELECT task_id FROM agent_tasks WHERE status IN ('completed', 'error', 'cancelled') AND completed_at < datetime('now', ?))";
        let days_param = format!("-{} days", days);

        let result = sqlx::query(sql)
            .bind(&days_param)
            .bind(&days_param)
            .execute(self.database.pool())
            .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait::async_trait]
impl Repository<AgentExecutionLog> for AgentExecutionLogRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentExecutionLog::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<AgentExecutionLog>> {
        let sql = "SELECT * FROM agent_execution_log ORDER BY timestamp DESC LIMIT 1000";
        let rows = sqlx::query(sql).fetch_all(self.database.pool()).await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(AgentExecutionLog::from_row(&row)?);
        }

        Ok(logs)
    }

    async fn save(&self, entity: &AgentExecutionLog) -> AppResult<i64> {
        self.create(entity).await
    }

    async fn update(&self, _entity: &AgentExecutionLog) -> AppResult<()> {
        // 执行日志通常不更新，只追加
        Ok(())
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let sql = "DELETE FROM agent_execution_log WHERE id = ?";
        sqlx::query(sql)
            .bind(id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }
}
