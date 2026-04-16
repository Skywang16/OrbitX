use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use sqlx::{self, Row};

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::rollout::{replay_rollout, RolloutItem};
use crate::agent::types::{Block, Message, MessageRole, MessageStatus};
use crate::storage::database::DatabaseManager;

use super::models::Workspace;
use super::timestamp_to_datetime;

static NEXT_MESSAGE_ID: AtomicI64 = AtomicI64::new(1_000_000);

#[derive(Debug)]
pub struct WorkspaceRepository {
    database: Arc<DatabaseManager>,
}

impl WorkspaceRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    pub async fn get(&self, path: &str) -> AgentResult<Option<Workspace>> {
        let row = sqlx::query(
            "SELECT path, display_name, active_thread_id, created_at, updated_at, last_accessed_at
             FROM workspaces WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(self.pool())
        .await?;

        row.map(|r| build_workspace(&r)).transpose()
    }
}

pub struct CreateMessageParams<'a> {
    pub thread_id: i64,
    pub role: MessageRole,
    pub status: MessageStatus,
    pub blocks: Vec<Block>,
    pub is_summary: bool,
    pub is_internal: bool,
    pub agent_type: &'a str,
    pub parent_message_id: Option<i64>,
    pub model_id: Option<&'a str>,
    pub provider_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct MessageRepository {
    database: Arc<DatabaseManager>,
}

impl MessageRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    async fn rollout_path(&self, thread_id: i64) -> AgentResult<String> {
        let row = sqlx::query("SELECT rollout_path FROM threads WHERE id = ?")
            .bind(thread_id)
            .fetch_optional(self.database.pool())
            .await?;

        row.map(|row| row.try_get::<String, _>("rollout_path"))
            .transpose()?
            .ok_or_else(|| AgentError::Internal(format!("thread {thread_id} not found")))
    }

    async fn load_messages(&self, thread_id: i64) -> AgentResult<Vec<Message>> {
        let rollout_path = self.rollout_path(thread_id).await?;
        let replayed = replay_rollout(Path::new(&rollout_path))
            .await
            .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        Ok(replayed.messages)
    }

    fn next_message_id() -> i64 {
        let seed = Utc::now()
            .timestamp_micros()
            .saturating_mul(100)
            .max(NEXT_MESSAGE_ID.load(Ordering::Relaxed));
        let _ = NEXT_MESSAGE_ID.fetch_max(seed, Ordering::SeqCst);
        NEXT_MESSAGE_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub async fn list_by_thread(&self, thread_id: i64) -> AgentResult<Vec<Message>> {
        self.load_messages(thread_id).await
    }

    pub async fn list_by_thread_paginated(
        &self,
        thread_id: i64,
        limit: i64,
        before_id: Option<i64>,
    ) -> AgentResult<Vec<Message>> {
        let mut messages = self.load_messages(thread_id).await?;
        if let Some(cursor) = before_id {
            messages.retain(|message| message.id < cursor);
        }
        if limit != i64::MAX && messages.len() > limit as usize {
            messages = messages[messages.len() - limit as usize..].to_vec();
        }
        Ok(messages)
    }

    pub async fn create(&self, params: CreateMessageParams<'_>) -> AgentResult<Message> {
        let created_at = Utc::now();
        let message = Message {
            id: Self::next_message_id(),
            thread_id: params.thread_id,
            role: params.role.clone(),
            agent_type: params.agent_type.to_string(),
            parent_message_id: params.parent_message_id,
            status: params.status,
            blocks: params.blocks,
            is_summary: params.is_summary,
            is_internal: params.is_internal,
            model_id: params.model_id.map(ToOwned::to_owned),
            provider_id: params.provider_id.map(ToOwned::to_owned),
            created_at,
            finished_at: None,
            duration_ms: None,
            token_usage: None,
            context_usage: None,
        };

        let item = match message.role {
            MessageRole::User => RolloutItem::UserMessageCreated {
                run_id: "native".to_string(),
                message: message.clone(),
            },
            MessageRole::Assistant => RolloutItem::AssistantMessageCreated {
                run_id: "native".to_string(),
                message: message.clone(),
            },
        };

        let recorder = crate::agent::rollout::RolloutRecorder::new(Arc::clone(&self.database));
        recorder
            .append(params.thread_id, item)
            .await
            .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;

        sqlx::query(
            "UPDATE threads
             SET updated_at = ?, last_event_at = ?, status = COALESCE(?, status),
                 first_user_message = COALESCE(?, first_user_message)
             WHERE id = ?",
        )
        .bind(created_at.timestamp())
        .bind(created_at.timestamp())
        .bind(if matches!(message.role, MessageRole::User) {
            Some("running")
        } else {
            None
        })
        .bind(first_user_text(&message))
        .bind(params.thread_id)
        .execute(self.database.pool())
        .await?;

        Ok(message)
    }

    pub async fn create_summary_message(
        &self,
        thread_id: i64,
        agent_type: &str,
        created_at_ts: i64,
    ) -> AgentResult<Message> {
        let message = Message {
            id: Self::next_message_id(),
            thread_id,
            role: MessageRole::Assistant,
            agent_type: agent_type.to_string(),
            parent_message_id: None,
            status: MessageStatus::Streaming,
            blocks: Vec::new(),
            is_summary: true,
            is_internal: false,
            model_id: None,
            provider_id: None,
            created_at: timestamp_to_datetime(created_at_ts),
            finished_at: None,
            duration_ms: None,
            token_usage: None,
            context_usage: None,
        };

        crate::agent::rollout::RolloutRecorder::new(Arc::clone(&self.database))
            .append(
                thread_id,
                RolloutItem::AssistantMessageCreated {
                    run_id: "summary".to_string(),
                    message: message.clone(),
                },
            )
            .await
            .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;

        Ok(message)
    }

    pub async fn update(&self, message: &Message) -> AgentResult<()> {
        let current = self
            .load_messages(message.thread_id)
            .await?
            .into_iter()
            .find(|candidate| candidate.id == message.id);

        let Some(existing) = current else {
            return Err(AgentError::Internal(format!(
                "message {} not found in thread {}",
                message.id, message.thread_id
            )));
        };

        let recorder = crate::agent::rollout::RolloutRecorder::new(Arc::clone(&self.database));

        for block in message.blocks.iter().skip(existing.blocks.len()) {
            recorder
                .append(
                    message.thread_id,
                    RolloutItem::BlockAppended {
                        run_id: "native".to_string(),
                        message_id: message.id,
                        block: block.clone(),
                    },
                )
                .await
                .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        }

        for (old_block, new_block) in existing.blocks.iter().zip(message.blocks.iter()) {
            if old_block == new_block {
                continue;
            }
            let Some(block_id) = block_id_of(new_block) else {
                continue;
            };
            recorder
                .append(
                    message.thread_id,
                    RolloutItem::BlockUpdated {
                        run_id: "native".to_string(),
                        message_id: message.id,
                        block_id,
                        block: new_block.clone(),
                    },
                )
                .await
                .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        }

        if message.status != existing.status
            || message.finished_at != existing.finished_at
            || message.duration_ms != existing.duration_ms
            || message.token_usage != existing.token_usage
        {
            recorder
                .append(
                    message.thread_id,
                    RolloutItem::MessageFinished {
                        run_id: "native".to_string(),
                        message_id: message.id,
                        status: message.status.clone(),
                        finished_at: message.finished_at.unwrap_or_else(Utc::now),
                        duration_ms: message.duration_ms.unwrap_or(0),
                        token_usage: message.token_usage.clone(),
                    },
                )
                .await
                .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        }

        Ok(())
    }

    pub async fn list_messages_from(
        &self,
        thread_id: i64,
        message_id: i64,
    ) -> AgentResult<Vec<Message>> {
        Ok(self
            .load_messages(thread_id)
            .await?
            .into_iter()
            .filter(|message| message.id >= message_id)
            .collect())
    }

    pub async fn delete_messages_from(&self, thread_id: i64, message_id: i64) -> AgentResult<()> {
        let rollout_path = self.rollout_path(thread_id).await?;
        let content = tokio::fs::read_to_string(&rollout_path)
            .await
            .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        let mut kept = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let decoded: crate::agent::rollout::RolloutLine = serde_json::from_str(line)
                .map_err(|err| AgentError::Parse(format!("Invalid rollout line: {err}")))?;
            if should_drop_rollout_item(&decoded.item, message_id) {
                continue;
            }
            kept.push(line.to_string());
        }

        let rewritten = if kept.is_empty() {
            String::new()
        } else {
            format!("{}\n", kept.join("\n"))
        };

        tokio::fs::write(&rollout_path, rewritten)
            .await
            .map_err(|err| AgentError::Io(std::io::Error::other(err.to_string())))?;
        Ok(())
    }
}

fn should_drop_rollout_item(item: &crate::agent::rollout::RolloutItem, message_id: i64) -> bool {
    match item {
        RolloutItem::UserMessageCreated { message, .. }
        | RolloutItem::AssistantMessageCreated { message, .. } => message.id >= message_id,
        RolloutItem::BlockAppended { message_id: id, .. }
        | RolloutItem::BlockUpdated { message_id: id, .. }
        | RolloutItem::MessageFinished { message_id: id, .. } => *id >= message_id,
        _ => false,
    }
}

fn block_id_of(block: &Block) -> Option<String> {
    match block {
        Block::Thinking(block) => Some(block.id.clone()),
        Block::Text(block) => Some(block.id.clone()),
        Block::Tool(block) => Some(block.id.clone()),
        _ => None,
    }
}

fn first_user_text(message: &Message) -> Option<String> {
    if !matches!(message.role, MessageRole::User) {
        return None;
    }
    message.blocks.iter().find_map(|block| match block {
        Block::UserText(text) => Some(text.content.clone()),
        _ => None,
    })
}

fn build_workspace(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Workspace> {
    Ok(Workspace {
        path: row.try_get("path")?,
        display_name: row.try_get("display_name")?,
        active_thread_id: row.try_get("active_thread_id")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
        last_accessed_at: timestamp_to_datetime(row.try_get::<i64, _>("last_accessed_at")?),
    })
}
