use std::sync::Arc;

use sqlx::{sqlite::SqliteRow, Row};

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::persistence::now_timestamp;
use crate::agent::ui::models::{UiConversation, UiMessage, UiMessageImage, UiStep};
use crate::storage::database::DatabaseManager;

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
    ) -> AgentResult<()> {
        let resolved_title = match title {
            Some(value) => Some(value.to_string()),
            None => self.fetch_conversation_title(conversation_id).await?,
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
    ) -> AgentResult<()> {
        sqlx::query("UPDATE agent_ui_conversations SET title = ? WHERE id = ?")
            .bind(title)
            .bind(conversation_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn list_conversations(&self) -> AgentResult<Vec<UiConversation>> {
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

    pub async fn get_messages(&self, conversation_id: i64) -> AgentResult<Vec<UiMessage>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, steps_json, status, duration_ms, created_at, images_json
             FROM agent_ui_messages
             WHERE conversation_id = ?
             ORDER BY created_at ASC, id ASC",
        )
        .bind(conversation_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(build_ui_message).collect()
    }

    pub async fn create_user_message(
        &self,
        conversation_id: i64,
        content: &str,
        images: Option<&[UiMessageImage]>,
    ) -> AgentResult<i64> {
        self.ensure_conversation(conversation_id, None).await?;
        let ts = now_timestamp();
        let preview = build_conversation_preview(content);
        let images_json = images.map(|imgs| serde_json::to_string(imgs).unwrap_or_default());
        let result = sqlx::query(
            "INSERT INTO agent_ui_messages (conversation_id, role, content, steps_json, status, duration_ms, created_at, images_json)
             VALUES (?, 'user', ?, NULL, NULL, NULL, ?, ?)",
        )
        .bind(conversation_id)
        .bind(content)
        .bind(ts)
        .bind(images_json)
        .execute(self.pool())
        .await?;

        self.touch_conversation(conversation_id, ts, preview)
            .await?;
        Ok(result.last_insert_rowid())
    }

    /// Create a fresh assistant message row for a new turn, returns the message id.
    pub async fn create_assistant_message(
        &self,
        conversation_id: i64,
        status: &str,
    ) -> AgentResult<i64> {
        self.ensure_conversation(conversation_id, None).await?;

        let normalized_status = match status {
            "streaming" | "complete" | "error" => status,
            other => {
                return Err(AgentError::Internal(format!(
                    "Invalid assistant message status: {}",
                    other
                )))
            }
        };

        let ts = now_timestamp();
        let result = sqlx::query(
            "INSERT INTO agent_ui_messages (conversation_id, role, content, steps_json, status, duration_ms, created_at)
             VALUES (?, 'assistant', NULL, '[]', ?, NULL, ?)",
        )
        .bind(conversation_id)
        .bind(normalized_status)
        .bind(ts)
        .execute(self.pool())
        .await?;

        // Touch conversation without preview at creation time
        self.touch_conversation(conversation_id, ts, None).await?;

        Ok(result.last_insert_rowid())
    }

    /// Update an existing assistant message by id with new steps and status.
    pub async fn update_assistant_message(
        &self,
        message_id: i64,
        steps: &[UiStep],
        status: &str,
    ) -> AgentResult<()> {
        let normalized_status = match status {
            "streaming" | "complete" | "error" => status,
            other => {
                return Err(AgentError::Internal(format!(
                    "Invalid assistant message status: {}",
                    other
                )))
            }
        };

        // Serialize steps and compute content preview from the latest non-empty text step
        let steps_json = serde_json::to_string(steps)?;
        let content_preview = steps
            .iter()
            .rev()
            .find(|step| step.step_type == "text" && !step.content.trim().is_empty())
            .map(|step| step.content.clone());

        let ts = now_timestamp();

        let mut tx = self.pool().begin().await?;

        // Fetch existing row info for duration calculation and conversation id
        let row = sqlx::query(
            "SELECT conversation_id, created_at, duration_ms FROM agent_ui_messages WHERE id = ?",
        )
        .bind(message_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AgentError::Internal(format!("Assistant message not found: {}", e)))?;

        let conversation_id: i64 = row.try_get("conversation_id")?;
        let created_at: i64 = row.try_get("created_at")?;
        let mut duration_ms: Option<i64> = row.try_get("duration_ms").ok();

        if (normalized_status == "complete" || normalized_status == "error")
            && duration_ms.is_none()
        {
            duration_ms = Some(ts.saturating_sub(created_at));
        }

        sqlx::query(
            "UPDATE agent_ui_messages
             SET steps_json = ?, status = ?, duration_ms = ?, content = ?
             WHERE id = ?",
        )
        .bind(&steps_json)
        .bind(normalized_status)
        .bind(duration_ms)
        .bind(content_preview.as_deref())
        .bind(message_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Update conversation timestamp/count only; do not override title with assistant content
        self.touch_conversation(conversation_id, ts, None).await?;

        Ok(())
    }

    pub async fn get_latest_assistant_message(
        &self,
        conversation_id: i64,
    ) -> AgentResult<Option<UiMessage>> {
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

    async fn fetch_conversation_title(&self, conversation_id: i64) -> AgentResult<Option<String>> {
        let title =
            sqlx::query_scalar::<_, Option<String>>("SELECT title FROM conversations WHERE id = ?")
                .bind(conversation_id)
                .fetch_optional(self.pool())
                .await?
                .flatten();

        Ok(title)
    }

    /// 删除指定消息及其之后的所有消息（用于回滚功能）
    /// 返回被删除的消息内容（如果是用户消息）以及回滚的时间边界
    pub async fn delete_messages_from(
        &self,
        conversation_id: i64,
        message_id: i64,
    ) -> AgentResult<MessageDeletionResult> {
        // 先获取该消息的内容（如果是用户消息）
        let message_content: Option<String> = sqlx::query_scalar(
            "SELECT content FROM agent_ui_messages WHERE id = ? AND role = 'user'",
        )
        .bind(message_id)
        .fetch_optional(self.pool())
        .await?
        .flatten();

        // 获取该消息的创建时间
        let created_at: i64 =
            sqlx::query_scalar("SELECT created_at FROM agent_ui_messages WHERE id = ?")
                .bind(message_id)
                .fetch_one(self.pool())
                .await?;

        // 删除该消息及其之后的所有消息
        sqlx::query(
            "DELETE FROM agent_ui_messages 
             WHERE conversation_id = ? AND (created_at > ? OR (created_at = ? AND id >= ?))",
        )
        .bind(conversation_id)
        .bind(created_at)
        .bind(created_at)
        .bind(message_id)
        .execute(self.pool())
        .await?;

        // 更新会话的消息计数
        let ts = now_timestamp();
        self.touch_conversation(conversation_id, ts, None).await?;

        Ok(MessageDeletionResult {
            user_content: message_content,
            cutoff_timestamp: created_at,
        })
    }

    async fn touch_conversation(
        &self,
        conversation_id: i64,
        timestamp: i64,
        preview: Option<String>,
    ) -> AgentResult<()> {
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

#[derive(Debug)]
pub struct MessageDeletionResult {
    pub user_content: Option<String>,
    pub cutoff_timestamp: i64,
}

fn build_conversation_preview(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    // 折叠多余空白，提取前 N 个可见字符
    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    const MAX_LEN: usize = 30;
    let mut preview = String::new();
    let mut count = 0;
    for ch in collapsed.chars() {
        if count >= MAX_LEN {
            preview.push_str("...");
            return Some(preview);
        }
        preview.push(ch);
        count += 1;
    }
    Some(preview)
}

fn build_ui_message(row: SqliteRow) -> AgentResult<UiMessage> {
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

    let images_json: Option<String> = row.try_get("images_json").unwrap_or(None);
    let images = match images_json {
        Some(raw) => {
            if raw.is_empty() {
                None
            } else {
                serde_json::from_str::<Vec<UiMessageImage>>(&raw).ok()
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
        images,
    })
}
