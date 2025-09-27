/*!
 * Agent工具调用Repository实现
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::storage::repositories::agent::{AgentToolCall, ToolCallStatus};
use crate::utils::error::AppResult;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

/// Agent工具调用Repository
pub struct AgentToolCallRepository {
    database: Arc<DatabaseManager>,
}

impl AgentToolCallRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 创建工具调用记录
    pub async fn create(&self, tool_call: &AgentToolCall) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("agent_tool_calls")
            .set("task_id", Value::String(tool_call.task_id.clone()))
            .set("call_id", Value::String(tool_call.call_id.clone()))
            .set("tool_name", Value::String(tool_call.tool_name.clone()))
            .set(
                "arguments_json",
                Value::String(tool_call.arguments_json.clone()),
            )
            .set(
                "result_json",
                tool_call
                    .result_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "status",
                Value::String(tool_call.status.as_str().to_string()),
            )
            .set(
                "error_message",
                tool_call
                    .error_message
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "started_at",
                Value::String(tool_call.started_at.to_rfc3339()),
            )
            .set(
                "completed_at",
                tool_call
                    .completed_at
                    .as_ref()
                    .map(|dt| Value::String(dt.to_rfc3339()))
                    .unwrap_or(Value::Null),
            )
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
                Value::Null => query.bind(None::<String>),
                _ => query.bind(param.to_string()),
            };
        }

        let result = query.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    /// 根据call_id查找工具调用
    pub async fn find_by_call_id(&self, call_id: &str) -> AppResult<Option<AgentToolCall>> {
        let sql = "SELECT * FROM agent_tool_calls WHERE call_id = ?";
        let row = sqlx::query(sql)
            .bind(call_id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentToolCall::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 根据任务ID查找工具调用
    pub async fn find_by_task_id(&self, task_id: &str) -> AppResult<Vec<AgentToolCall>> {
        sqlx::query("SELECT * FROM agent_tool_calls WHERE task_id = ? ORDER BY started_at ASC")
            .bind(task_id)
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentToolCall::from_row(&row))
            .collect()
    }

    /// 更新工具调用状态
    pub async fn update_status(
        &self,
        call_id: &str,
        status: ToolCallStatus,
        result: Option<Value>,
        error_message: Option<String>,
    ) -> AppResult<()> {
        let sql = match status {
            ToolCallStatus::Running => {
                "UPDATE agent_tool_calls SET status = ? WHERE call_id = ?"
            }
            ToolCallStatus::Completed | ToolCallStatus::Error => {
                "UPDATE agent_tool_calls SET status = ?, result_json = ?, error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE call_id = ?"
            }
            _ => "UPDATE agent_tool_calls SET status = ? WHERE call_id = ?",
        };

        match status {
            ToolCallStatus::Completed | ToolCallStatus::Error => {
                sqlx::query(sql)
                    .bind(status.as_str())
                    .bind(result.map(|r| r.to_string()))
                    .bind(error_message)
                    .bind(call_id)
                    .execute(self.database.pool())
                    .await?;
            }
            _ => {
                sqlx::query(sql)
                    .bind(status.as_str())
                    .bind(call_id)
                    .execute(self.database.pool())
                    .await?;
            }
        }

        Ok(())
    }

    /// 更新工具调用结果
    pub async fn update_result(
        &self,
        call_id: &str,
        result: Option<Value>,
        status: ToolCallStatus,
    ) -> AppResult<()> {
        let result_json = result.map(|r| r.to_string());

        let sql = "UPDATE agent_tool_calls SET result_json = ?, status = ?, completed_at = CURRENT_TIMESTAMP WHERE call_id = ?";

        sqlx::query(sql)
            .bind(result_json)
            .bind(status.as_str())
            .bind(call_id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 更新工具调用错误
    pub async fn update_error(&self, call_id: &str, error_message: &str) -> AppResult<()> {
        let sql = "UPDATE agent_tool_calls SET status = ?, error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE call_id = ?";

        sqlx::query(sql)
            .bind(ToolCallStatus::Error.as_str())
            .bind(error_message)
            .bind(call_id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 根据状态查找工具调用
    pub async fn find_by_status(&self, status: ToolCallStatus) -> AppResult<Vec<AgentToolCall>> {
        sqlx::query("SELECT * FROM agent_tool_calls WHERE status = ? ORDER BY started_at DESC")
            .bind(status.as_str())
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentToolCall::from_row(&row))
            .collect()
    }

    /// 根据工具名称查找调用记录
    pub async fn find_by_tool_name(&self, tool_name: &str) -> AppResult<Vec<AgentToolCall>> {
        sqlx::query(
            "SELECT * FROM agent_tool_calls WHERE tool_name = ? ORDER BY started_at DESC LIMIT 100",
        )
        .bind(tool_name)
        .fetch_all(self.database.pool())
        .await?
        .into_iter()
        .map(|row| AgentToolCall::from_row(&row))
        .collect()
    }

    /// 获取工具使用统计
    pub async fn get_tool_usage_stats(&self) -> AppResult<Vec<(String, i64)>> {
        sqlx::query("SELECT tool_name, COUNT(*) as usage_count FROM agent_tool_calls GROUP BY tool_name ORDER BY usage_count DESC")
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| -> AppResult<(String, i64)> {
                Ok((row.try_get("tool_name")?, row.try_get("usage_count")?))
            })
            .collect()
    }

    /// 清理过期的工具调用记录
    pub async fn cleanup_old_calls(&self, days: i32) -> AppResult<u64> {
        let sql = "DELETE FROM agent_tool_calls WHERE started_at < datetime('now', ?) AND task_id IN (SELECT task_id FROM agent_tasks WHERE status IN ('completed', 'error', 'cancelled') AND completed_at < datetime('now', ?))";
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
impl Repository<AgentToolCall> for AgentToolCallRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<AgentToolCall>> {
        let sql = "SELECT * FROM agent_tool_calls WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentToolCall::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<AgentToolCall>> {
        sqlx::query("SELECT * FROM agent_tool_calls ORDER BY started_at DESC LIMIT 1000")
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentToolCall::from_row(&row))
            .collect()
    }

    async fn save(&self, entity: &AgentToolCall) -> AppResult<i64> {
        self.create(entity).await
    }

    async fn update(&self, entity: &AgentToolCall) -> AppResult<()> {
        self.update_status(
            &entity.call_id,
            entity.status.clone(),
            entity
                .result_json
                .as_ref()
                .map(|s| serde_json::from_str(s).unwrap_or(serde_json::Value::Null)),
            entity.error_message.clone(),
        )
        .await
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let sql = "DELETE FROM agent_tool_calls WHERE id = ?";
        sqlx::query(sql)
            .bind(id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }
}
