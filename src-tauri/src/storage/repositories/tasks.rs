use crate::storage::database::DatabaseManager;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde_json::Value;
use sqlx::{Executor, Row};
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskRepository {
    db: Arc<DatabaseManager>,
}

/// 任务列表查询过滤器（模块级导出，供外部调用）
pub struct TaskListFilter {
    pub status: Option<String>,
    pub parent_task_id: Option<String>,
    pub root_task_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_desc: bool,
}

impl Default for TaskListFilter {
    fn default() -> Self {
        Self {
            status: None,
            parent_task_id: None,
            root_task_id: None,
            limit: Some(50),
            offset: Some(0),
            order_desc: true,
        }
    }
}

impl TaskRepository {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }

    pub async fn upsert_task_index(
        &self,
        task_id: &str,
        name: Option<&str>,
        status: Option<&str>,
        parent_task_id: Option<&str>,
        root_task_id: Option<&str>,
        metadata: Option<&Value>,
    ) -> AppResult<()> {
        let metadata_json = metadata.map(|v| v.to_string());
        let sql = r#"
            INSERT INTO task_index (task_id, name, status, parent_task_id, root_task_id, metadata_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, COALESCE((SELECT created_at FROM task_index WHERE task_id = ?), CURRENT_TIMESTAMP), CURRENT_TIMESTAMP)
            ON CONFLICT(task_id) DO UPDATE SET
              name=COALESCE(excluded.name, task_index.name),
              status=COALESCE(excluded.status, task_index.status),
              parent_task_id=COALESCE(excluded.parent_task_id, task_index.parent_task_id),
              root_task_id=COALESCE(excluded.root_task_id, task_index.root_task_id),
              metadata_json=COALESCE(excluded.metadata_json, task_index.metadata_json),
              updated_at=CURRENT_TIMESTAMP
        "#;

        sqlx::query(sql)
            .bind(task_id)
            .bind(name)
            .bind(status)
            .bind(parent_task_id)
            .bind(root_task_id)
            .bind(metadata_json)
            .bind(task_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    pub async fn replace_ui_events(&self, task_id: &str, events: &[Value]) -> AppResult<()> {
        let mut tx = self.db.pool().begin().await?;
        sqlx::query("DELETE FROM task_ui_events WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;

        let insert_sql = r#"
            INSERT INTO task_ui_events (task_id, event_json, ts_ms)
            VALUES (?, ?, ?)
        "#;
        for ev in events {
            let ts_ms = ev
                .get("_ts")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
            sqlx::query(insert_sql)
                .bind(task_id)
                .bind(ev.to_string())
                .bind(ts_ms)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn read_ui_events(&self, task_id: &str) -> AppResult<Vec<Value>> {
        let rows = sqlx::query(
            r#"SELECT event_json FROM task_ui_events WHERE task_id = ? ORDER BY id ASC"#,
        )
        .bind(task_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let json_str: String = row.try_get("event_json").map_err(|e| anyhow!(e))?;
            let v: Value = serde_json::from_str(&json_str)?;
            out.push(v);
        }
        Ok(out)
    }

    pub async fn save_api_messages(&self, task_id: &str, messages: &Value) -> AppResult<()> {
        let sql = r#"
            INSERT INTO task_api_messages (task_id, messages_json, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(task_id) DO UPDATE SET
              messages_json = excluded.messages_json,
              updated_at = CURRENT_TIMESTAMP
        "#;
        sqlx::query(sql)
            .bind(task_id)
            .bind(messages.to_string())
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    pub async fn read_api_messages(&self, task_id: &str) -> AppResult<Value> {
        let row = sqlx::query(
            r#"SELECT messages_json FROM task_api_messages WHERE task_id = ? LIMIT 1"#,
        )
        .bind(task_id)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(row) = row {
            let json_str: Option<String> = row.try_get("messages_json").map_err(|e| anyhow!(e))?;
            if let Some(s) = json_str {
                Ok(serde_json::from_str(&s)?)
            } else {
                Ok(Value::Null)
            }
        } else {
            Ok(Value::Null)
        }
    }

    pub async fn save_metadata(&self, task_id: &str, metadata: &Value) -> AppResult<()> {
        self.upsert_task_index(task_id, None, None, None, None, Some(metadata))
            .await
    }

    pub async fn read_metadata(&self, task_id: &str) -> AppResult<Value> {
        let row = sqlx::query(r#"SELECT metadata_json FROM task_index WHERE task_id = ?"#)
            .bind(task_id)
            .fetch_optional(self.db.pool())
            .await?;
        if let Some(row) = row {
            let json_str: Option<String> = row.try_get("metadata_json").map_err(|e| anyhow!(e))?;
            if let Some(s) = json_str { Ok(serde_json::from_str(&s)?) } else { Ok(Value::Null) }
        } else {
            Ok(Value::Null)
        }
    }

    pub async fn save_checkpoint(
        &self,
        task_id: &str,
        name: Option<&str>,
        checkpoint: &Value,
    ) -> AppResult<String> {
        let name = if let Some(n) = name {
            if n.trim().is_empty() {
                chrono::Utc::now().timestamp_millis().to_string()
            } else {
                n.to_string()
            }
        } else {
            chrono::Utc::now().timestamp_millis().to_string()
        };

        let sql = r#"
            INSERT INTO task_checkpoints (task_id, name, checkpoint_json, created_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(task_id, name) DO UPDATE SET
              checkpoint_json = excluded.checkpoint_json,
              created_at = CURRENT_TIMESTAMP
        "#;
        sqlx::query(sql)
            .bind(task_id)
            .bind(&name)
            .bind(checkpoint.to_string())
            .execute(self.db.pool())
            .await?;
        Ok(name)
    }

    pub async fn list_checkpoints(&self, task_id: &str) -> AppResult<Vec<String>> {
        let rows = sqlx::query(
            r#"SELECT name FROM task_checkpoints WHERE task_id = ? ORDER BY created_at ASC"#,
        )
        .bind(task_id)
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.try_get("name").map_err(|e| anyhow!(e))?;
            out.push(name);
        }
        Ok(out)
    }

    pub async fn purge_all(&self) -> AppResult<()> {
        let mut tx = self.db.pool().begin().await?;
        tx.execute("DELETE FROM task_checkpoints").await?;
        tx.execute("DELETE FROM task_ui_events").await?;
        tx.execute("DELETE FROM task_api_messages").await?;
        tx.execute("DELETE FROM task_index").await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_task(&self, task_id: &str) -> AppResult<()> {
        let mut tx = self.db.pool().begin().await?;
        sqlx::query("DELETE FROM task_checkpoints WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM task_ui_events WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM task_api_messages WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM task_index WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> AppResult<Option<serde_json::Value>> {
        let row = sqlx::query(
            r#"
            SELECT task_id, name, status, parent_task_id, root_task_id, metadata_json, created_at, updated_at
            FROM task_index WHERE task_id = ? LIMIT 1
            "#,
        )
        .bind(task_id)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(row) = row {
            let task_id: String = row.try_get("task_id").map_err(|e| anyhow!(e))?;
            let name: Option<String> = row.try_get("name").map_err(|e| anyhow!(e))?;
            let status: Option<String> = row.try_get("status").map_err(|e| anyhow!(e))?;
            let parent_task_id: Option<String> = row.try_get("parent_task_id").map_err(|e| anyhow!(e))?;
            let root_task_id: Option<String> = row.try_get("root_task_id").map_err(|e| anyhow!(e))?;
            let metadata_json: Option<String> = row.try_get("metadata_json").map_err(|e| anyhow!(e))?;
            let created_at: String = row.try_get("created_at").map_err(|e| anyhow!(e))?;
            let updated_at: String = row.try_get("updated_at").map_err(|e| anyhow!(e))?;

            let mut obj = serde_json::Map::new();
            obj.insert("taskId".into(), serde_json::Value::String(task_id));
            obj.insert(
                "name".into(),
                name.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "status".into(),
                status.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "parentTaskId".into(),
                parent_task_id
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "rootTaskId".into(),
                root_task_id
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            let meta_v = if let Some(s) = metadata_json {
                serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };
            obj.insert("metadata".into(), meta_v);
            obj.insert("createdAt".into(), serde_json::Value::String(created_at));
            obj.insert("updatedAt".into(), serde_json::Value::String(updated_at));

            Ok(Some(serde_json::Value::Object(obj)))
        } else {
            Ok(None)
        }
    }
    pub async fn list_tasks(&self, filter: TaskListFilter) -> AppResult<Vec<serde_json::Value>> {
        let mut sql = String::from(
            "SELECT task_id, name, status, parent_task_id, root_task_id, metadata_json, created_at, updated_at FROM task_index",
        );
        let mut conditions: Vec<String> = Vec::new();
        let mut binds: Vec<serde_json::Value> = Vec::new();

        if let Some(s) = &filter.status {
            conditions.push("status = ?".into());
            binds.push(serde_json::Value::String(s.clone()));
        }
        if let Some(p) = &filter.parent_task_id {
            conditions.push("parent_task_id = ?".into());
            binds.push(serde_json::Value::String(p.clone()));
        }
        if let Some(r) = &filter.root_task_id {
            conditions.push("root_task_id = ?".into());
            binds.push(serde_json::Value::String(r.clone()));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY updated_at ");
        sql.push_str(if filter.order_desc { "DESC" } else { "ASC" });

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&limit.to_string());
        }
        if let Some(offset) = filter.offset {
            sql.push_str(" OFFSET ");
            sql.push_str(&offset.to_string());
        }

        let mut query = sqlx::query(&sql);
        for b in binds {
            match b {
                serde_json::Value::String(s) => {
                    query = query.bind(s);
                }
                _ => {}
            }
        }

        let rows = query.fetch_all(self.db.pool()).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let task_id: String = row.try_get("task_id").map_err(|e| anyhow!(e))?;
            let name: Option<String> = row.try_get("name").map_err(|e| anyhow!(e))?;
            let status: Option<String> = row.try_get("status").map_err(|e| anyhow!(e))?;
            let parent_task_id: Option<String> = row.try_get("parent_task_id").map_err(|e| anyhow!(e))?;
            let root_task_id: Option<String> = row.try_get("root_task_id").map_err(|e| anyhow!(e))?;
            let metadata_json: Option<String> = row.try_get("metadata_json").map_err(|e| anyhow!(e))?;
            let created_at: String = row.try_get("created_at").map_err(|e| anyhow!(e))?;
            let updated_at: String = row.try_get("updated_at").map_err(|e| anyhow!(e))?;

            let mut obj = serde_json::Map::new();
            obj.insert("taskId".into(), serde_json::Value::String(task_id));
            obj.insert(
                "name".into(),
                name.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "status".into(),
                status.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "parentTaskId".into(),
                parent_task_id
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            obj.insert(
                "rootTaskId".into(),
                root_task_id
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            let meta_v = if let Some(s) = metadata_json {
                serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };
            obj.insert("metadata".into(), meta_v);
            obj.insert("createdAt".into(), serde_json::Value::String(created_at));
            obj.insert("updatedAt".into(), serde_json::Value::String(updated_at));
            out.push(serde_json::Value::Object(obj));
        }

        Ok(out)
    }
}
