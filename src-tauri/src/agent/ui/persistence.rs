use std::sync::Arc;

use anyhow::{anyhow, Context};
use sqlx::{sqlite::SqliteRow, Row};

use crate::agent::persistence::now_timestamp;
use crate::agent::ui::models::{UiConversation, UiMessage, UiStep};
use crate::storage::database::DatabaseManager;
use crate::utils::error::AppResult;

#[derive(Debug)]
pub struct AgentUiPersistence {
    database: Arc<DatabaseManager>,
}

impl AgentUiPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    /// Ensure a UI conversation record exists (id is shared with core conversations table).
    pub async fn ensure_conversation(
        &self,
        conversation_id: i64,
        title: Option<&str>,
    ) -> AppResult<()> {
        let resolved_title = match title {
            Some(value) => Some(value.to_string()),
            None => self
                .fetch_conversation_title(conversation_id)
                .await
                .context("fetching conversation title")?,
        };

        let inserted_at = now_timestamp();
        sqlx::query(
            "INSERT INTO agent_ui_conversations (id, title, message_count, updated_at)
             VALUES (?, ?, 0, ?)
             ON CONFLICT(id) DO UPDATE SET
                title = COALESCE(excluded.title, agent_ui_conversations.title)",
        )
        .bind(conversation_id)
        .bind(resolved_title)
        .bind(inserted_at)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    /// Update the stored title for a conversation.
    pub async fn update_conversation_title(
        &self,
        conversation_id: i64,
        title: &str,
    ) -> AppResult<()> {
        sqlx::query("UPDATE agent_ui_conversations SET title = ? WHERE id = ?")
            .bind(title)
            .bind(conversation_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_conversations(&self) -> AppResult<Vec<UiConversation>> {
        let rows = sqlx::query(
            "SELECT c.id AS conversation_id,
                    COALESCE(NULLIF(u.title, ''), NULLIF(c.title, '')) AS title,
                    COALESCE(u.message_count, 0) AS message_count,
                    COALESCE(u.updated_at, c.updated_at) AS updated_at,
                    c.created_at AS created_at
             FROM conversations c
             LEFT JOIN agent_ui_conversations u ON u.id = c.id
             ORDER BY updated_at DESC, conversation_id DESC",
        )
        .fetch_all(self.pool())
        .await?;

        let conversations = rows
            .into_iter()
            .map(|row| UiConversation {
                id: row.try_get("conversation_id").unwrap_or_default(),
                title: row.try_get("title").unwrap_or(None),
                message_count: row.try_get::<i64, _>("message_count").unwrap_or(0),
                created_at: row
                    .try_get::<Option<i64>, _>("created_at")
                    .unwrap_or(None)
                    .unwrap_or_else(now_timestamp),
                updated_at: row
                    .try_get::<Option<i64>, _>("updated_at")
                    .unwrap_or(None)
                    .unwrap_or_else(now_timestamp),
            })
            .collect();

        Ok(conversations)
    }

    pub async fn get_messages(&self, conversation_id: i64) -> AppResult<Vec<UiMessage>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, steps_json, status, duration_ms, created_at
             FROM agent_ui_messages
             WHERE conversation_id = ?
             ORDER BY created_at ASC, id ASC",
        )
        .bind(conversation_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(build_ui_message).collect()
    }

    pub async fn create_user_message(&self, conversation_id: i64, content: &str) -> AppResult<i64> {
        self.ensure_conversation(conversation_id, None).await?;
        let ts = now_timestamp();
        let preview = build_conversation_preview(content);
        let result = sqlx::query(
            "INSERT INTO agent_ui_messages (conversation_id, role, content, steps_json, status, duration_ms, created_at)
             VALUES (?, 'user', ?, NULL, NULL, NULL, ?)",
        )
        .bind(conversation_id)
        .bind(content)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.touch_conversation(conversation_id, ts, preview)
            .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn upsert_assistant_message(
        &self,
        conversation_id: i64,
        steps: &[UiStep],
        status: &str,
    ) -> AppResult<i64> {
        self.ensure_conversation(conversation_id, None).await?;

        let normalized_status = match status {
            "streaming" | "complete" | "error" => status,
            other => return Err(anyhow!("invalid assistant message status: {}", other)),
        };

        let steps_json = serde_json::to_string(steps)?;
        let ts = now_timestamp();
        let content_preview = steps
            .iter()
            .rev()
            .find(|step| step.step_type == "text" && !step.content.trim().is_empty())
            .map(|step| step.content.clone());

        let mut tx = self.pool().begin().await?;

        let existing = sqlx::query(
            "SELECT id, status, duration_ms, created_at
             FROM agent_ui_messages
             WHERE conversation_id = ? AND role = 'assistant'
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await?;

        let message_id: i64 = match existing {
            Some(row) => {
                let existing_status = row.try_get::<Option<String>, _>("status")?;
                let mut existing_duration = row.try_get::<Option<i64>, _>("duration_ms")?;
                let id: i64 = row.try_get("id")?;
                let created_at: i64 = row.try_get("created_at")?;

                match existing_status.as_deref() {
                    Some("streaming") | Some("complete") => {
                        if (normalized_status == "complete" || normalized_status == "error")
                            && existing_duration.is_none()
                        {
                            existing_duration = Some(ts.saturating_sub(created_at));
                        }

                        sqlx::query(
                            "UPDATE agent_ui_messages
                             SET steps_json = ?, status = ?, duration_ms = ?, content = ?
                             WHERE id = ?",
                        )
                        .bind(&steps_json)
                        .bind(normalized_status)
                        .bind(existing_duration)
                        .bind(content_preview.as_deref())
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;

                        id
                    }
                    Some("error") => {
                        let result = sqlx::query(
                            "INSERT INTO agent_ui_messages (conversation_id, role, content, steps_json, status, duration_ms, created_at)
                             VALUES (?, 'assistant', ?, ?, ?, NULL, ?)",
                        )
                        .bind(conversation_id)
                        .bind(content_preview.as_deref())
                        .bind(&steps_json)
                        .bind(normalized_status)
                        .bind(ts)
                        .execute(&mut *tx)
                        .await?;

                        result.last_insert_rowid()
                    }
                    _ => {
                        if existing_duration.is_none() {
                            existing_duration = Some(ts.saturating_sub(created_at));
                        }

                        sqlx::query(
                            "UPDATE agent_ui_messages
                             SET steps_json = ?, status = ?, duration_ms = ?, content = ?
                             WHERE id = ?",
                        )
                        .bind(&steps_json)
                        .bind(normalized_status)
                        .bind(existing_duration)
                        .bind(content_preview.as_deref())
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;

                        id
                    }
                }
            }
            None => {
                let result = sqlx::query(
                    "INSERT INTO agent_ui_messages (conversation_id, role, content, steps_json, status, duration_ms, created_at)
                     VALUES (?, 'assistant', ?, ?, ?, NULL, ?)",
                )
                .bind(conversation_id)
                .bind(content_preview.as_deref())
                .bind(&steps_json)
                .bind(normalized_status)
                .bind(ts)
                .execute(&mut *tx)
                .await?;

                result.last_insert_rowid()
            }
        };

        tx.commit().await?;

        self.touch_conversation(conversation_id, ts, None).await?;

        Ok(message_id)
    }

    pub async fn get_latest_assistant_message(
        &self,
        conversation_id: i64,
    ) -> AppResult<Option<UiMessage>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, role, content, steps_json, status, duration_ms, created_at
             FROM agent_ui_messages
             WHERE conversation_id = ? AND role = 'assistant'
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(self.pool())
        .await?;

        match row {
            Some(row) => Ok(Some(build_ui_message(row)?)),
            None => Ok(None),
        }
    }

    async fn fetch_conversation_title(&self, conversation_id: i64) -> AppResult<Option<String>> {
        let title =
            sqlx::query_scalar::<_, Option<String>>("SELECT title FROM conversations WHERE id = ?")
                .bind(conversation_id)
                .fetch_optional(self.pool())
                .await?
                .flatten();

        Ok(title)
    }

    async fn touch_conversation(
        &self,
        conversation_id: i64,
        timestamp: i64,
        preview: Option<String>,
    ) -> AppResult<()> {
        let preview_ref = preview.as_deref();
        sqlx::query(
            "UPDATE agent_ui_conversations
             SET message_count = (
                     SELECT COUNT(*)
                     FROM agent_ui_messages
                     WHERE conversation_id = ?
                 ),
                 updated_at = ?,
                 title = CASE WHEN ? IS NULL THEN title ELSE ? END
             WHERE id = ?",
        )
        .bind(conversation_id)
        .bind(timestamp)
        .bind(preview_ref)
        .bind(preview_ref)
        .bind(conversation_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

fn build_conversation_preview(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }

    let mut preview = String::new();
    let mut count = 0;
    for ch in collapsed.chars() {
        if count >= 120 {
            preview.push_str("...");
            return Some(preview);
        }
        preview.push(ch);
        count += 1;
    }

    Some(preview)
}

fn build_ui_message(row: SqliteRow) -> AppResult<UiMessage> {
    let steps_json: Option<String> = row.try_get("steps_json")?;
    let steps = match steps_json {
        Some(raw) => {
            if raw.is_empty() {
                None
            } else {
                Some(serde_json::from_str::<Vec<UiStep>>(&raw)?)
            }
        }
        None => None,
    };

    Ok(UiMessage {
        id: row.try_get("id")?,
        conversation_id: row.try_get("conversation_id")?,
        role: row.try_get("role")?,
        content: row.try_get("content")?,
        steps,
        status: row.try_get("status")?,
        duration_ms: row.try_get("duration_ms")?,
        created_at: row.try_get("created_at")?,
    })
}
