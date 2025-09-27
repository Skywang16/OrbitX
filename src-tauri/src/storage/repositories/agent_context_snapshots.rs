/*!
 * Agent上下文快照Repository实现
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::storage::repositories::agent::{AgentContextSnapshot, ContextType};
use crate::utils::error::AppResult;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

/// Agent上下文快照Repository
pub struct AgentContextSnapshotRepository {
    database: Arc<DatabaseManager>,
}

impl AgentContextSnapshotRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 创建上下文快照
    pub async fn create(&self, snapshot: &AgentContextSnapshot) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("agent_context_snapshots")
            .set("task_id", Value::String(snapshot.task_id.clone()))
            .set("iteration", Value::Number(snapshot.iteration.into()))
            .set(
                "context_type",
                Value::String(snapshot.context_type.as_str().to_string()),
            )
            .set(
                "messages_json",
                Value::String(snapshot.messages_json.clone()),
            )
            .set(
                "additional_state_json",
                snapshot
                    .additional_state_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "created_at",
                Value::String(snapshot.created_at.to_rfc3339()),
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

    /// 根据任务ID查找快照
    pub async fn find_by_task_id(&self, task_id: &str) -> AppResult<Vec<AgentContextSnapshot>> {
        sqlx::query("SELECT * FROM agent_context_snapshots WHERE task_id = ? ORDER BY iteration DESC, created_at DESC")
            .bind(task_id)
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentContextSnapshot::from_row(&row))
            .collect()
    }

    /// 根据任务ID和迭代号查找快照
    pub async fn find_by_task_and_iteration(
        &self,
        task_id: &str,
        iteration: u32,
    ) -> AppResult<Option<AgentContextSnapshot>> {
        let sql = "SELECT * FROM agent_context_snapshots WHERE task_id = ? AND iteration = ? ORDER BY created_at DESC LIMIT 1";
        let row = sqlx::query(sql)
            .bind(task_id)
            .bind(iteration as i64)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentContextSnapshot::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 获取最新的快照
    pub async fn get_latest_snapshot(
        &self,
        task_id: &str,
    ) -> AppResult<Option<AgentContextSnapshot>> {
        let sql = "SELECT * FROM agent_context_snapshots WHERE task_id = ? ORDER BY iteration DESC, created_at DESC LIMIT 1";
        let row = sqlx::query(sql)
            .bind(task_id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentContextSnapshot::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 根据上下文类型查找快照
    pub async fn find_by_context_type(
        &self,
        task_id: &str,
        context_type: ContextType,
    ) -> AppResult<Vec<AgentContextSnapshot>> {
        sqlx::query("SELECT * FROM agent_context_snapshots WHERE task_id = ? AND context_type = ? ORDER BY iteration DESC, created_at DESC")
            .bind(task_id)
            .bind(context_type.as_str())
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentContextSnapshot::from_row(&row))
            .collect()
    }

    /// 创建完整快照
    pub async fn create_full_snapshot(
        &self,
        task_id: &str,
        iteration: u32,
        messages: &Value,
        additional_state: Option<&Value>,
    ) -> AppResult<i64> {
        let mut snapshot = AgentContextSnapshot::new(
            task_id.to_string(),
            iteration,
            ContextType::Full,
            messages.clone(),
        );

        if let Some(state) = additional_state {
            snapshot = snapshot.with_additional_state(state.clone());
        }

        self.create(&snapshot).await
    }

    /// 创建增量快照
    pub async fn create_incremental_snapshot(
        &self,
        task_id: &str,
        iteration: u32,
        messages: &Value,
        additional_state: Option<&Value>,
    ) -> AppResult<i64> {
        let mut snapshot = AgentContextSnapshot::new(
            task_id.to_string(),
            iteration,
            ContextType::Incremental,
            messages.clone(),
        );

        if let Some(state) = additional_state {
            snapshot = snapshot.with_additional_state(state.clone());
        }

        self.create(&snapshot).await
    }

    /// 重建完整上下文（从快照恢复）
    pub async fn rebuild_context(
        &self,
        task_id: &str,
        target_iteration: u32,
    ) -> AppResult<Option<Value>> {
        // 1. 查找最近的完整快照
        let full_snapshot_sql = "SELECT * FROM agent_context_snapshots WHERE task_id = ? AND context_type = 'full' AND iteration <= ? ORDER BY iteration DESC LIMIT 1";
        let full_snapshot_row = sqlx::query(full_snapshot_sql)
            .bind(task_id)
            .bind(target_iteration as i64)
            .fetch_optional(self.database.pool())
            .await?;

        let (mut context, base_iteration) = if let Some(row) = full_snapshot_row {
            let snapshot = AgentContextSnapshot::from_row(&row)?;
            let base_iter = snapshot.iteration as i64;
            let ctx = serde_json::from_str::<Value>(&snapshot.messages_json)?;
            (ctx, base_iter)
        } else {
            // 如果没有完整快照，返回空上下文
            return Ok(None);
        };

        // 2. 应用后续的增量快照
        let incremental_sql = "SELECT * FROM agent_context_snapshots WHERE task_id = ? AND context_type = 'incremental' AND iteration > ? AND iteration <= ? ORDER BY iteration ASC";

        let incremental_rows = sqlx::query(incremental_sql)
            .bind(task_id)
            .bind(base_iteration)
            .bind(target_iteration as i64)
            .fetch_all(self.database.pool())
            .await?;

        for row in incremental_rows {
            let snapshot = AgentContextSnapshot::from_row(&row)?;
            let incremental_data = serde_json::from_str::<Value>(&snapshot.messages_json)?;

            // 合并增量数据到上下文中
            if let (Value::Array(base_array), Value::Array(incremental_array)) =
                (&mut context, incremental_data)
            {
                base_array.extend(incremental_array);
            }
        }

        Ok(Some(context))
    }

    /// 清理过期的快照（保留每个任务最新的5个快照）
    pub async fn cleanup_old_snapshots(&self) -> AppResult<u64> {
        let sql = "DELETE FROM agent_context_snapshots WHERE id NOT IN (SELECT id FROM agent_context_snapshots GROUP BY task_id HAVING COUNT(*) <= 5) AND task_id IN (SELECT task_id FROM agent_tasks WHERE status IN ('completed', 'error', 'cancelled'))";

        let result = sqlx::query(sql).execute(self.database.pool()).await?;

        Ok(result.rows_affected())
    }

    /// 根据任务ID删除所有快照
    pub async fn delete_by_task_id(&self, task_id: &str) -> AppResult<u64> {
        let sql = "DELETE FROM agent_context_snapshots WHERE task_id = ?";
        let result = sqlx::query(sql)
            .bind(task_id)
            .execute(self.database.pool())
            .await?;

        Ok(result.rows_affected())
    }

    /// 获取快照统计信息
    pub async fn get_snapshot_stats(&self, task_id: &str) -> AppResult<(i64, i64)> {
        let sql = "SELECT COUNT(*) as total_count, COUNT(CASE WHEN context_type = 'full' THEN 1 END) as full_count FROM agent_context_snapshots WHERE task_id = ?";
        let row = sqlx::query(sql)
            .bind(task_id)
            .fetch_one(self.database.pool())
            .await?;

        let total_count: i64 = row.try_get("total_count")?;
        let full_count: i64 = row.try_get("full_count")?;

        Ok((total_count, full_count))
    }
}

#[async_trait::async_trait]
impl Repository<AgentContextSnapshot> for AgentContextSnapshotRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<AgentContextSnapshot>> {
        let sql = "SELECT * FROM agent_context_snapshots WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentContextSnapshot::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<AgentContextSnapshot>> {
        sqlx::query("SELECT * FROM agent_context_snapshots ORDER BY created_at DESC LIMIT 100")
            .fetch_all(self.database.pool())
            .await?
            .into_iter()
            .map(|row| AgentContextSnapshot::from_row(&row))
            .collect()
    }

    async fn save(&self, entity: &AgentContextSnapshot) -> AppResult<i64> {
        self.create(entity).await
    }

    async fn update(&self, _entity: &AgentContextSnapshot) -> AppResult<()> {
        // 快照通常不更新，只追加
        Ok(())
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let sql = "DELETE FROM agent_context_snapshots WHERE id = ?";
        sqlx::query(sql)
            .bind(id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }
}
