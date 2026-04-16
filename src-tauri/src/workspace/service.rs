use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use sqlx::{self, Row};

use crate::agent::persistence::AgentPersistence;
use crate::agent::rollout::replay_rollout;
use crate::agent::subagent::SubagentRepository;
use crate::agent::types::{Block, Message, SubagentRecord};
use crate::storage::DatabaseManager;

use super::error::{WorkspaceError, WorkspaceResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub path: String,
    pub display_name: Option<String>,
    pub active_thread_id: Option<i64>,
    pub selected_run_action_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunActionRecord {
    pub id: String,
    pub workspace_path: String,
    pub name: String,
    pub command: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRecord {
    pub id: i64,
    pub workspace_path: String,
    pub parent_thread_id: Option<i64>,
    pub title: String,
    pub message_count: i64,
    pub status: String,
    pub thread_type: String,
    pub agent_type: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionNodeRecord {
    pub id: i64,
    pub backing_thread_id: Option<i64>,
    pub role: String,
    pub profile: String,
    pub title: String,
    pub status: String,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub children: Vec<ExecutionNodeRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadViewRecord {
    pub thread: ThreadRecord,
    pub timeline: Vec<ThreadTimelineItemRecord>,
    pub execution_tree: Vec<ExecutionNodeRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadTimelineItemRecord {
    pub id: String,
    pub message_id: i64,
    pub title: String,
    pub created_at: i64,
    pub status: Option<String>,
}

pub struct WorkspaceService {
    database: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
}

impl WorkspaceService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let persistence = Arc::new(AgentPersistence::new(Arc::clone(&database)));
        Self {
            database,
            agent_persistence: persistence,
        }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    fn now_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    async fn normalize_path(&self, path: &str) -> WorkspaceResult<String> {
        if path.trim().is_empty() {
            return Err(WorkspaceError::invalid_path("Path cannot be empty"));
        }
        let original = path.to_string();
        tokio::task::spawn_blocking(move || -> WorkspaceResult<String> {
            let candidate = PathBuf::from(&original);
            let canonical = if candidate.exists() {
                std::fs::canonicalize(&candidate).map_err(|e| {
                    WorkspaceError::invalid_path(format!("Canonicalize failed: {e}"))
                })?
            } else {
                candidate
            };
            path_to_string(&canonical)
        })
        .await
        .map_err(|e| WorkspaceError::internal(format!("Task join error: {e}")))?
    }

    pub async fn get_or_create_workspace(&self, path: &str) -> WorkspaceResult<WorkspaceRecord> {
        let normalized = self.normalize_path(path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "INSERT INTO workspaces (path, display_name, active_thread_id, created_at, updated_at, last_accessed_at)
             VALUES (?, NULL, NULL, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at",
        )
        .bind(&normalized)
        .bind(ts)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get_workspace(&normalized)
            .await?
            .ok_or_else(|| WorkspaceError::workspace_not_found(&normalized))
    }

    pub async fn list_recent_workspaces(
        &self,
        limit: i64,
    ) -> WorkspaceResult<Vec<WorkspaceRecord>> {
        let rows = sqlx::query(
            "SELECT path, display_name, active_thread_id, selected_run_action_id, created_at, updated_at, last_accessed_at
             FROM workspaces
             ORDER BY last_accessed_at DESC LIMIT ?",
        )
        .bind(limit.max(1))
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(build_workspace).collect()
    }

    pub async fn list_threads(&self, workspace_path: &str) -> WorkspaceResult<Vec<ThreadRecord>> {
        let normalized = self.normalize_path(workspace_path).await?;
        let rows = sqlx::query(
            "SELECT id, workspace_path, parent_thread_id, title, status, thread_type, agent_type, created_at, updated_at
             FROM threads
             WHERE workspace_path = ? AND is_archived = 0
             ORDER BY updated_at DESC, id DESC",
        )
        .bind(&normalized)
        .fetch_all(self.pool())
        .await?;

        let mut threads = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let message_count = self.get_thread_messages(id, i64::MAX, None).await?.len() as i64;
            threads.push(ThreadRecord {
                id,
                workspace_path: row.try_get("workspace_path")?,
                parent_thread_id: row.try_get("parent_thread_id")?,
                title: row.try_get("title")?,
                status: row.try_get("status")?,
                thread_type: row.try_get("thread_type")?,
                agent_type: row.try_get("agent_type")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                message_count,
            });
        }

        Ok(threads)
    }

    pub async fn list_thread_views(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<Vec<ThreadViewRecord>> {
        let workspace_threads = self.list_threads(workspace_path).await?;
        let top_level_threads = workspace_threads
            .iter()
            .filter(|thread| thread.parent_thread_id.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let mut views = Vec::with_capacity(top_level_threads.len());

        for thread in top_level_threads {
            views.push(
                self.build_thread_view(thread, workspace_threads.clone())
                    .await?,
            );
        }

        Ok(views)
    }

    pub async fn create_thread(
        &self,
        workspace_path: &str,
        title: &str,
        thread_type: &str,
    ) -> WorkspaceResult<ThreadRecord> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        let agent_type_value = "coder"; // agent_type 用于 agent 子类型
        let thread = self
            .agent_persistence
            .threads()
            .create(crate::agent::rollout::projection::CreateThreadParams {
                workspace_path: &workspace.path,
                title,
                display_name: None,
                thread_type,
                agent_type: agent_type_value,
                parent_thread_id: None,
                spawned_by_tool_call_id: None,
                rollout_path: "",
                worktree_path: None,
                model_id: None,
                provider_id: None,
            })
            .await
            .map_err(|e| {
                WorkspaceError::internal(format!("Create thread projection failed: {e}"))
            })?;
        let real_meta = crate::agent::rollout::ThreadMeta {
            thread_id: thread.id,
            workspace_path: workspace.path.clone(),
            title: title.to_string(),
            thread_type: thread_type.to_string(),
            agent_type: agent_type_value.to_string(),
            parent_thread_id: None,
            spawned_by_tool_call_id: None,
            model_id: None,
            provider_id: None,
            created_at: Utc::now(),
        };
        let rollout_path = self
            .agent_persistence
            .rollout_recorder()
            .ensure_thread_rollout(&real_meta)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Create thread rollout failed: {e}")))?;
        self.agent_persistence
            .threads()
            .update_rollout_path(thread.id, &rollout_path.to_string_lossy())
            .await
            .map_err(|e| WorkspaceError::internal(format!("Update rollout path failed: {e}")))?;

        self.get_thread(thread.id)
            .await?
            .ok_or_else(|| WorkspaceError::thread_not_found(thread.id))
    }

    pub async fn get_active_thread(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<Option<ThreadRecord>> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        match workspace.active_thread_id {
            Some(thread_id) => self.get_thread(thread_id).await,
            None => Ok(None),
        }
    }

    pub async fn ensure_active_thread(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<ThreadRecord> {
        self.ensure_active_thread_with_title(workspace_path, "")
            .await
    }

    pub async fn ensure_active_thread_with_title(
        &self,
        workspace_path: &str,
        title: &str,
    ) -> WorkspaceResult<ThreadRecord> {
        if let Some(thread) = self.get_active_thread(workspace_path).await? {
            if !thread.title.trim().is_empty() || title.trim().is_empty() {
                return Ok(thread);
            }
            self.update_thread_title(thread.id, title).await?;
            return self
                .get_thread(thread.id)
                .await?
                .ok_or_else(|| WorkspaceError::thread_not_found(thread.id));
        }

        let thread_title = if title.trim().is_empty() { "" } else { title };
        let created = self.create_thread(workspace_path, thread_title, "agent").await?;
        self.set_active_thread(workspace_path, Some(created.id)).await?;
        Ok(created)
    }

    pub async fn update_thread_title(&self, thread_id: i64, title: &str) -> WorkspaceResult<()> {
        let ts = Self::now_timestamp();
        sqlx::query("UPDATE threads SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(ts)
            .bind(thread_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn refresh_thread_title(&self, thread_id: i64) -> WorkspaceResult<()> {
        let messages = self.get_thread_messages(thread_id, i64::MAX, None).await?;
        let title = messages
            .iter()
            .rev()
            .find_map(|message| {
                message.blocks.iter().find_map(|block| match block {
                    Block::UserText(text) => {
                        let normalized = normalize_timeline_title(&text.content);
                        (!normalized.is_empty()).then_some(normalized)
                    }
                    _ => None,
                })
            })
            .unwrap_or_default();
        let ts = messages
            .last()
            .map(|message| message.created_at.timestamp())
            .unwrap_or_else(Self::now_timestamp);
        sqlx::query("UPDATE threads SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(ts)
            .bind(thread_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn set_active_thread(
        &self,
        workspace_path: &str,
        thread_id: Option<i64>,
    ) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "UPDATE workspaces
             SET active_thread_id = ?, updated_at = ?, last_accessed_at = ?
             WHERE path = ?",
        )
        .bind(thread_id)
        .bind(ts)
        .bind(ts)
        .bind(&normalized)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn get_thread_messages(
        &self,
        thread_id: i64,
        limit: i64,
        before_id: Option<i64>,
    ) -> WorkspaceResult<Vec<Message>> {
        let thread = self
            .agent_persistence
            .threads()
            .get(thread_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Load thread projection failed: {e}")))?
            .ok_or_else(|| WorkspaceError::thread_not_found(thread_id))?;
        let replayed = replay_rollout(std::path::Path::new(&thread.rollout_path))
            .await
            .map_err(|e| WorkspaceError::internal(format!("Replay rollout failed: {e}")))?;
        let mut messages = replayed.messages;
        if let Some(cursor) = before_id {
            messages.retain(|message| message.id < cursor);
        }
        if limit != i64::MAX && messages.len() > limit as usize {
            messages = messages[messages.len() - limit as usize..].to_vec();
        }
        Ok(messages)
    }

    pub async fn list_thread_timeline(
        &self,
        thread_id: i64,
    ) -> WorkspaceResult<Vec<ThreadTimelineItemRecord>> {
        let messages = self.get_thread_messages(thread_id, i64::MAX, None).await?;
        let mut user_messages = messages
            .into_iter()
            .filter(|message| matches!(message.role, crate::agent::types::MessageRole::User))
            .collect::<Vec<_>>();
        user_messages.sort_by_key(|message| message.created_at);

        let thread = self
            .get_thread(thread_id)
            .await?
            .ok_or_else(|| WorkspaceError::thread_not_found(thread_id))?;

        Ok(user_messages
            .into_iter()
            .map(|message| ThreadTimelineItemRecord {
                id: format!("message-{}", message.id),
                message_id: message.id,
                title: message
                    .blocks
                    .iter()
                    .find_map(|block| match block {
                        Block::UserText(text) => Some(normalize_timeline_title(&text.content)),
                        _ => None,
                    })
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| "Untitled message".to_string()),
                created_at: message.created_at.timestamp(),
                status: Some(thread.status.clone()),
            })
            .collect())
    }

    pub async fn delete_thread(&self, thread_id: i64) -> WorkspaceResult<()> {
        let rollout_dir = self
            .agent_persistence
            .rollout_recorder()
            .thread_dir(thread_id);
        sqlx::query("DELETE FROM threads WHERE id = ?")
            .bind(thread_id)
            .execute(self.pool())
            .await?;
        if tokio::fs::try_exists(&rollout_dir).await.unwrap_or(false) {
            tokio::fs::remove_dir_all(&rollout_dir).await.map_err(|e| {
                WorkspaceError::internal(format!("Delete thread rollout failed: {e}"))
            })?;
        }
        Ok(())
    }

    pub async fn delete_workspace(&self, path: &str) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(path).await?;
        let thread_ids = sqlx::query("SELECT id FROM threads WHERE workspace_path = ?")
            .bind(&normalized)
            .fetch_all(self.pool())
            .await?;
        for row in thread_ids {
            let thread_id: i64 = row.try_get("id")?;
            let rollout_dir = self
                .agent_persistence
                .rollout_recorder()
                .thread_dir(thread_id);
            if tokio::fs::try_exists(&rollout_dir).await.unwrap_or(false) {
                let _ = tokio::fs::remove_dir_all(&rollout_dir).await;
            }
        }
        sqlx::query("DELETE FROM workspaces WHERE path = ?")
            .bind(&normalized)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn maintain(
        &self,
        max_age_days: i64,
        max_entries: i64,
    ) -> WorkspaceResult<(u64, u64)> {
        let cutoff = Self::now_timestamp() - max_age_days * 24 * 60 * 60;

        let deleted_expired = sqlx::query("DELETE FROM workspaces WHERE last_accessed_at < ?")
            .bind(cutoff)
            .execute(self.pool())
            .await?
            .rows_affected();

        let total_workspaces = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.pool())
            .await?;
        let excess = total_workspaces.saturating_sub(max_entries);

        if excess > 0 {
            sqlx::query(
                "DELETE FROM workspaces WHERE path IN (
                    SELECT path FROM workspaces
                    ORDER BY last_accessed_at DESC
                    LIMIT -1 OFFSET ?
                )",
            )
            .bind(max_entries)
            .execute(self.pool())
            .await?;
        }

        Ok((deleted_expired, excess.max(0) as u64))
    }

    async fn get_workspace(&self, path: &str) -> WorkspaceResult<Option<WorkspaceRecord>> {
        let row = sqlx::query(
            "SELECT path, display_name, active_thread_id, selected_run_action_id, created_at, updated_at, last_accessed_at
             FROM workspaces WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(self.pool())
        .await?;

        row.map(build_workspace).transpose()
    }

    pub async fn get_thread(&self, id: i64) -> WorkspaceResult<Option<ThreadRecord>> {
        let row = sqlx::query(
            "SELECT id, workspace_path, parent_thread_id, title, status, thread_type, agent_type, created_at, updated_at
             FROM threads WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let message_count = self.get_thread_messages(id, i64::MAX, None).await?.len() as i64;
        Ok(Some(ThreadRecord {
            id,
            workspace_path: row.try_get("workspace_path")?,
            parent_thread_id: row.try_get("parent_thread_id")?,
            title: row.try_get("title")?,
            status: row.try_get("status")?,
            thread_type: row.try_get("thread_type")?,
            agent_type: row.try_get("agent_type")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            message_count,
        }))
    }

    async fn build_thread_view(
        &self,
        thread: ThreadRecord,
        workspace_threads: Vec<ThreadRecord>,
    ) -> WorkspaceResult<ThreadViewRecord> {
        Ok(ThreadViewRecord {
            timeline: self.list_thread_timeline(thread.id).await?,
            execution_tree: build_execution_tree(&workspace_threads, Some(thread.id)),
            thread,
        })
    }

    pub async fn trim_thread_messages(
        &self,
        _workspace_path: &str,
        thread_id: i64,
        message_id: i64,
    ) -> WorkspaceResult<()> {
        let subagent_repo = SubagentRepository::new(Arc::clone(&self.database));
        let stale_subagents = subagent_repo
            .delete_from_parent_message(thread_id, message_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Trim thread subagents failed: {e}")))?;

        for subagent in stale_subagents {
            self.delete_thread(subagent.child_thread_id).await?;
        }

        self.agent_persistence
            .messages()
            .delete_messages_from(thread_id, message_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Trim thread rollout failed: {e}")))?;
        self.refresh_thread_title(thread_id).await?;
        Ok(())
    }

    pub async fn list_subagents(
        &self,
        parent_thread_id: i64,
    ) -> WorkspaceResult<Vec<SubagentRecord>> {
        SubagentRepository::new(Arc::clone(&self.database))
            .list_by_parent_thread(parent_thread_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("List subagents failed: {e}")))
    }
}

fn build_execution_tree(
    threads: &[ThreadRecord],
    root_thread_id: Option<i64>,
) -> Vec<ExecutionNodeRecord> {
    fn build_children(
        threads: &[ThreadRecord],
        parent_id: Option<i64>,
    ) -> Vec<ExecutionNodeRecord> {
        let mut children = threads
            .iter()
            .filter(|thread| thread.parent_thread_id == parent_id)
            .cloned()
            .collect::<Vec<_>>();
        children.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        children
            .into_iter()
            .map(|thread| ExecutionNodeRecord {
                id: thread.id,
                backing_thread_id: Some(thread.id),
                role: if thread.parent_thread_id.is_none() {
                    "root".to_string()
                } else {
                    "branch".to_string()
                },
                profile: thread.agent_type.clone(),
                title: thread.title.clone(),
                status: thread.status.clone(),
                started_at: Some(thread.created_at),
                finished_at: matches!(thread.status.as_str(), "completed" | "error" | "cancelled")
                    .then_some(thread.updated_at),
                children: build_children(threads, Some(thread.id)),
            })
            .collect()
    }

    match root_thread_id {
        Some(id) => threads
            .iter()
            .find(|thread| thread.id == id)
            .map(|thread| ExecutionNodeRecord {
                id: thread.id,
                backing_thread_id: Some(thread.id),
                role: if thread.parent_thread_id.is_none() {
                    "root".to_string()
                } else {
                    "branch".to_string()
                },
                profile: thread.agent_type.clone(),
                title: thread.title.clone(),
                status: thread.status.clone(),
                started_at: Some(thread.created_at),
                finished_at: matches!(thread.status.as_str(), "completed" | "error" | "cancelled")
                    .then_some(thread.updated_at),
                children: build_children(threads, Some(thread.id)),
            })
            .into_iter()
            .collect(),
        None => build_children(threads, None),
    }
}

fn path_to_string(path: &Path) -> WorkspaceResult<String> {
    let display = path
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    Ok(display)
}

fn build_workspace(row: sqlx::sqlite::SqliteRow) -> WorkspaceResult<WorkspaceRecord> {
    Ok(WorkspaceRecord {
        path: row.try_get("path")?,
        display_name: row.try_get("display_name")?,
        active_thread_id: row.try_get("active_thread_id")?,
        selected_run_action_id: row.try_get("selected_run_action_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_accessed_at: row.try_get("last_accessed_at")?,
    })
}

fn normalize_timeline_title(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.len() <= 72 {
        return trimmed.to_string();
    }
    let mut truncated = trimmed.chars().take(72).collect::<String>();
    truncated.push_str("...");
    truncated
}
