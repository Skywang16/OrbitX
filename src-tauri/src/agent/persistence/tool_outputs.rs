use std::sync::Arc;

use sqlx::{self, Row};

use crate::agent::error::AgentResult;
use crate::storage::database::DatabaseManager;

use super::now_timestamp;

#[derive(Debug)]
pub struct ToolOutputRepository {
    database: Arc<DatabaseManager>,
}

impl ToolOutputRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn upsert(
        &self,
        session_id: i64,
        message_id: i64,
        block_id: &str,
        tool_name: &str,
        output_content: &str,
    ) -> AgentResult<()> {
        let ts = now_timestamp();
        sqlx::query(
            "INSERT INTO tool_outputs (
                session_id, message_id, block_id, tool_name, output_content, compacted_at, created_at
             ) VALUES (?, ?, ?, ?, ?, NULL, ?)
             ON CONFLICT(message_id, block_id) DO UPDATE SET
                tool_name = excluded.tool_name,
                output_content = excluded.output_content",
        )
        .bind(session_id)
        .bind(message_id)
        .bind(block_id)
        .bind(tool_name)
        .bind(output_content)
        .bind(ts)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn list_by_message_ids(
        &self,
        message_ids: &[i64],
    ) -> AgentResult<Vec<(i64, String, String, Option<i64>)>> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = message_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT message_id, block_id, output_content, compacted_at
             FROM tool_outputs
             WHERE message_id IN ({placeholders})"
        );

        let mut query = sqlx::query(&sql);
        for id in message_ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(self.pool()).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<i64, _>("message_id").unwrap_or_default(),
                    row.try_get::<String, _>("block_id").unwrap_or_default(),
                    row.try_get::<String, _>("output_content")
                        .unwrap_or_default(),
                    row.try_get::<Option<i64>, _>("compacted_at")
                        .unwrap_or(None),
                )
            })
            .collect())
    }

    pub async fn mark_compacted(
        &self,
        session_id: i64,
        message_id: i64,
        block_id: &str,
        compacted_at: i64,
    ) -> AgentResult<()> {
        sqlx::query(
            "UPDATE tool_outputs
             SET compacted_at = ?
             WHERE session_id = ? AND message_id = ? AND block_id = ?",
        )
        .bind(compacted_at)
        .bind(session_id)
        .bind(message_id)
        .bind(block_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn clear_compaction_marks_before(
        &self,
        session_id: i64,
        message_id: i64,
    ) -> AgentResult<()> {
        sqlx::query(
            "UPDATE tool_outputs
             SET compacted_at = NULL
             WHERE session_id = ? AND message_id <= ?",
        )
        .bind(session_id)
        .bind(message_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn clear_compaction_marks_for_session(&self, session_id: i64) -> AgentResult<()> {
        sqlx::query(
            "UPDATE tool_outputs
             SET compacted_at = NULL
             WHERE session_id = ?",
        )
        .bind(session_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}
