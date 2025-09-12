/*!
 * 会话Repository
 *
 * 处理AI会话和消息的数据访问逻辑
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

/// 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: Option<i64>,
    pub title: String,
    pub message_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            title,
            message_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

impl RowMapper<Conversation> for Conversation {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        Ok(Self {
            id: Some(row.try_get("id")?),
            title: row.try_get("title")?,
            message_count: row.try_get("message_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Option<i64>,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub steps_json: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

impl Message {
    pub fn new(conversation_id: i64, role: String, content: String) -> Self {
        Self {
            id: None,
            conversation_id,
            role,
            content,
            steps_json: None,
            status: None,
            duration_ms: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

impl RowMapper<Message> for Message {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let created_at_dt: DateTime<Utc> = row.try_get("created_at")?;
        let id: i64 = row.try_get("id")?;
        let role: String = row.try_get("role")?;
        let steps_json: Option<String> = row.try_get("steps_json")?;

        Ok(Self {
            id: Some(id),
            conversation_id: row.try_get("conversation_id")?,
            role,
            content: row.try_get("content")?,
            steps_json,
            status: row.try_get("status")?,
            duration_ms: row.try_get("duration_ms")?,
            created_at: created_at_dt.to_rfc3339(),
        })
    }
}

/// 会话Repository
pub struct ConversationRepository {
    database: Arc<DatabaseManager>,
}

impl ConversationRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 获取会话列表
    pub async fn find_conversations(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<Vec<Conversation>> {
        let mut builder = SafeQueryBuilder::new("ai_conversations")
            .select(&["id", "title", "message_count", "created_at", "updated_at"])
            .order_by(crate::storage::query::QueryOrder::Desc(
                "updated_at".to_string(),
            ));

        if let Some(limit) = limit {
            builder = builder.limit(limit);
        }

        if let Some(offset) = offset {
            builder = builder.offset(offset);
        }

        let (sql, params) = builder.build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("Unsupported number type"));
                    }
                }
                _ => return Err(anyhow!("Unsupported parameter type")),
            };
        }

        let rows = query_builder.fetch_all(self.database.pool()).await?;
        let conversations: Vec<Conversation> = rows
            .iter()
            .map(|row| Conversation::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conversations)
    }

    /// 更新会话标题
    pub async fn update_title(&self, conversation_id: i64, title: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE ai_conversations SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(title)
        .bind(conversation_id)
        .execute(self.database.pool())
        .await?;

        Ok(())
    }

    /// 保存消息
    pub async fn ai_conversation_save_message(&self, message: &Message) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("ai_messages")
            .set(
                "conversation_id",
                Value::Number(message.conversation_id.into()),
            )
            .set("role", Value::String(message.role.clone()))
            .set("content", Value::String(message.content.clone()))
            .set(
                "steps_json",
                message
                    .steps_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "status",
                message
                    .status
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "duration_ms",
                message
                    .duration_ms
                    .map(|d| Value::Number(d.into()))
                    .unwrap_or(Value::Null),
            )
            .set("created_at", Value::String(message.created_at.clone()))
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("Unsupported number type"));
                    }
                }
                Value::Null => query_builder.bind(None::<String>),
                _ => return Err(anyhow!("Unsupported parameter type")),
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    /// 获取会话消息
    pub async fn get_messages(
        &self,
        conversation_id: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<Vec<Message>> {
        let mut builder = SafeQueryBuilder::new("ai_messages")
            .select(&[
                "id",
                "conversation_id",
                "role",
                "content",
                "steps_json",
                "status",
                "duration_ms",
                "created_at",
            ])
            .where_condition(QueryCondition::Eq(
                "conversation_id".to_string(),
                Value::Number(conversation_id.into()),
            ))
            .order_by(crate::storage::query::QueryOrder::Asc(
                "created_at".to_string(),
            ));

        if let Some(limit) = limit {
            builder = builder.limit(limit);
        }

        if let Some(offset) = offset {
            builder = builder.offset(offset);
        }

        let (sql, params) = builder.build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("Unsupported number type"));
                    }
                }
                _ => return Err(anyhow!("Unsupported parameter type")),
            };
        }

        let rows = query_builder.fetch_all(self.database.pool()).await?;
        let messages: Vec<Message> = rows
            .iter()
            .map(|row| Message::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// 删除指定消息ID之后的所有消息
    pub async fn delete_messages_after(
        &self,
        conversation_id: i64,
        after_message_id: i64,
    ) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM ai_messages WHERE conversation_id = ? AND id > ?")
            .bind(conversation_id)
            .bind(after_message_id)
            .execute(self.database.pool())
            .await?;

        Ok(result.rows_affected())
    }

    /// 更新消息内容
    pub async fn ai_conversation_update_message_content(&self, message_id: i64, content: &str) -> AppResult<()> {
        sqlx::query("UPDATE ai_messages SET content = ? WHERE id = ?")
            .bind(content)
            .bind(message_id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 更新消息步骤数据
    pub async fn ai_conversation_update_message_steps(&self, message_id: i64, steps_json: &str) -> AppResult<()> {
        sqlx::query("UPDATE ai_messages SET steps_json = ? WHERE id = ?")
            .bind(steps_json)
            .bind(message_id)
            .execute(self.database.pool())
            .await?;
        Ok(())
    }

    /// 更新消息状态
    pub async fn ai_conversation_update_message_status(
        &self,
        message_id: i64,
        status: Option<&str>,
        duration_ms: Option<i64>,
    ) -> AppResult<()> {
        sqlx::query("UPDATE ai_messages SET status = ?, duration_ms = ? WHERE id = ?")
            .bind(status)
            .bind(duration_ms)
            .bind(message_id)
            .execute(self.database.pool())
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Repository<Conversation> for ConversationRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<Conversation>> {
        let (sql, _params) = SafeQueryBuilder::new("ai_conversations")
            .where_condition(QueryCondition::Eq(
                "id".to_string(),
                Value::Number(id.into()),
            ))
            .build()?;

        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(Conversation::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<Conversation>> {
        self.find_conversations(None, None).await
    }

    async fn save(&self, entity: &Conversation) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("ai_conversations")
            .set("title", Value::String(entity.title.clone()))
            .set("message_count", Value::Number(entity.message_count.into()))
            .set("created_at", Value::String(entity.created_at.to_rfc3339()))
            .set("updated_at", Value::String(entity.updated_at.to_rfc3339()))
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("Unsupported number type"));
                    }
                }
                _ => return Err(anyhow!("Unsupported parameter type")),
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, entity: &Conversation) -> AppResult<()> {
        if let Some(id) = entity.id {
            sqlx::query("UPDATE ai_conversations SET title = ?, message_count = ?, updated_at = ? WHERE id = ?")
                .bind(&entity.title)
                .bind(entity.message_count)
                .bind(entity.updated_at)
                .bind(id)
                .execute(self.database.pool())
                .await?;
            Ok(())
        } else {
            Err(anyhow!("无法更新没有ID的会话"))
        }
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("会话不存在: {}", id));
        }

        Ok(())
    }
}
