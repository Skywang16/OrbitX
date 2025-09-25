/*!
 * 任务系统Repository
 *
 * 处理任务系统的数据访问逻辑，实现四层架构的数据存储层
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

/// UI任务 - 用于UI渲染层
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UITask {
    pub ui_id: Option<i64>,
    pub conversation_id: i64,
    pub task_id: String,
    pub name: String,
    pub status: TaskStatus,
    pub parent_ui_id: Option<i64>,
    pub render_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UITask {
    pub fn new(conversation_id: i64, task_id: String, name: String, status: TaskStatus) -> Self {
        let now = Utc::now();
        Self {
            ui_id: None,
            conversation_id,
            task_id,
            name,
            status,
            parent_ui_id: None,
            render_json: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_parent(mut self, parent_ui_id: i64) -> Self {
        self.parent_ui_id = Some(parent_ui_id);
        self
    }

    pub fn with_render_data(mut self, render_json: String) -> Self {
        self.render_json = Some(render_json);
        self
    }
}

impl RowMapper<UITask> for UITask {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let status_str: String = row.try_get("status")?;
        let status = TaskStatus::from_str(&status_str)?;

        Ok(Self {
            ui_id: Some(row.try_get("ui_id")?),
            conversation_id: row.try_get("conversation_id")?,
            task_id: row.try_get("task_id")?,
            name: row.try_get("name")?,
            status,
            parent_ui_id: row.try_get("parent_ui_id")?,
            render_json: row.try_get("render_json")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Eko上下文 - 用于原始事件存储
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EkoContext {
    pub id: Option<i64>,
    pub task_id: String,
    pub conversation_id: i64,
    pub kind: EkoContextKind,
    pub name: Option<String>,
    pub node_id: Option<String>,
    pub status: Option<EkoStatus>,
    pub payload_json: String,
    pub created_at: DateTime<Utc>,
}

impl EkoContext {
    pub fn new(
        task_id: String,
        conversation_id: i64,
        kind: EkoContextKind,
        payload_json: String,
    ) -> Self {
        Self {
            id: None,
            task_id,
            conversation_id,
            kind,
            name: None,
            node_id: None,
            status: None,
            payload_json,
            created_at: Utc::now(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_node_id(mut self, node_id: String) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_status(mut self, status: EkoStatus) -> Self {
        self.status = Some(status);
        self
    }
}

impl RowMapper<EkoContext> for EkoContext {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let kind_str: String = row.try_get("kind")?;
        let kind = EkoContextKind::from_str(&kind_str)?;

        let status = if let Ok(status_str) = row.try_get::<Option<String>, _>("status") {
            if let Some(s) = status_str {
                Some(EkoStatus::from_str(&s)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            id: Some(row.try_get("id")?),
            task_id: row.try_get("task_id")?,
            conversation_id: row.try_get("conversation_id")?,
            kind,
            name: row.try_get("name")?,
            node_id: row.try_get("node_id")?,
            status,
            payload_json: row.try_get("payload_json")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// 任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Init,
    Active,
    Paused,
    Completed,
    Error,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "init" => Ok(Self::Init),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            _ => Err(anyhow!("无效的任务状态: {}", s)),
        }
    }
}

/// Eko上下文类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EkoContextKind {
    State,
    Event,
    Snapshot,
}

impl EkoContextKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Event => "event",
            Self::Snapshot => "snapshot",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "state" => Ok(Self::State),
            "event" => Ok(Self::Event),
            "snapshot" => Ok(Self::Snapshot),
            _ => Err(anyhow!("无效的Eko上下文类型: {}", s)),
        }
    }
}

/// Eko状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EkoStatus {
    Init,
    Running,
    Paused,
    Aborted,
    Done,
    Error,
}

impl EkoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Aborted => "aborted",
            Self::Done => "done",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "init" => Ok(Self::Init),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "aborted" => Ok(Self::Aborted),
            "done" => Ok(Self::Done),
            "error" => Ok(Self::Error),
            _ => Err(anyhow!("无效的Eko状态: {}", s)),
        }
    }
}

/// 任务Repository
pub struct TaskRepository {
    database: Arc<DatabaseManager>,
}

impl TaskRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 创建UI任务
    pub async fn create_ui_task(&self, task: &UITask) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("ui_tasks")
            .set(
                "conversation_id",
                Value::Number(task.conversation_id.into()),
            )
            .set("task_id", Value::String(task.task_id.clone()))
            .set("name", Value::String(task.name.clone()))
            .set("status", Value::String(task.status.as_str().to_string()))
            .set(
                "parent_ui_id",
                task.parent_ui_id
                    .map(|id| Value::Number(id.into()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "render_json",
                task.render_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set("created_at", Value::String(task.created_at.to_rfc3339()))
            .set("updated_at", Value::String(task.updated_at.to_rfc3339()))
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("不支持的数字类型"));
                    }
                }
                Value::Null => query_builder.bind(None::<String>),
                _ => return Err(anyhow!("不支持的参数类型")),
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    /// 更新UI任务
    pub async fn update_ui_task(&self, ui_id: i64, updates: &UITask) -> AppResult<()> {
        sqlx::query(
            "UPDATE ui_tasks SET name = ?, status = ?, render_json = ?, updated_at = ? WHERE ui_id = ?"
        )
        .bind(&updates.name)
        .bind(updates.status.as_str())
        .bind(&updates.render_json)
        .bind(updates.updated_at)
        .bind(ui_id)
        .execute(self.database.pool())
        .await?;

        Ok(())
    }

    /// 按会话查询UI任务
    pub async fn get_ui_tasks_by_conversation(
        &self,
        conversation_id: i64,
    ) -> AppResult<Vec<UITask>> {
        let (sql, _params) = SafeQueryBuilder::new("ui_tasks")
            .select(&[
                "ui_id",
                "conversation_id",
                "task_id",
                "name",
                "status",
                "parent_ui_id",
                "render_json",
                "created_at",
                "updated_at",
            ])
            .where_condition(QueryCondition::Eq(
                "conversation_id".to_string(),
                Value::Number(conversation_id.into()),
            ))
            .order_by(crate::storage::query::QueryOrder::Desc(
                "updated_at".to_string(),
            ))
            .build()?;

        let rows = sqlx::query(&sql)
            .bind(conversation_id)
            .fetch_all(self.database.pool())
            .await?;

        let tasks: Vec<UITask> = rows
            .iter()
            .map(|row| UITask::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// 删除UI任务
    pub async fn delete_ui_task(&self, ui_id: i64) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM ui_tasks WHERE ui_id = ?")
            .bind(ui_id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("UI任务不存在: {}", ui_id));
        }

        Ok(())
    }

    /// 保存Eko上下文
    pub async fn save_eko_context(&self, context: &EkoContext) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("eko_context")
            .set("task_id", Value::String(context.task_id.clone()))
            .set(
                "conversation_id",
                Value::Number(context.conversation_id.into()),
            )
            .set("kind", Value::String(context.kind.as_str().to_string()))
            .set(
                "name",
                context
                    .name
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "node_id",
                context
                    .node_id
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "status",
                context
                    .status
                    .as_ref()
                    .map(|s| Value::String(s.as_str().to_string()))
                    .unwrap_or(Value::Null),
            )
            .set("payload_json", Value::String(context.payload_json.clone()))
            .set("created_at", Value::String(context.created_at.to_rfc3339()))
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else {
                        return Err(anyhow!("不支持的数字类型"));
                    }
                }
                Value::Null => query_builder.bind(None::<String>),
                _ => return Err(anyhow!("不支持的参数类型")),
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    /// 获取Eko上下文
    pub async fn get_eko_contexts(&self, task_id: &str) -> AppResult<Vec<EkoContext>> {
        let (sql, _params) = SafeQueryBuilder::new("eko_context")
            .select(&[
                "id",
                "task_id",
                "conversation_id",
                "kind",
                "name",
                "node_id",
                "status",
                "payload_json",
                "created_at",
            ])
            .where_condition(QueryCondition::Eq(
                "task_id".to_string(),
                Value::String(task_id.to_string()),
            ))
            .order_by(crate::storage::query::QueryOrder::Asc(
                "created_at".to_string(),
            ))
            .build()?;

        let rows = sqlx::query(&sql)
            .bind(task_id)
            .fetch_all(self.database.pool())
            .await?;

        let contexts: Vec<EkoContext> = rows
            .iter()
            .map(|row| EkoContext::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(contexts)
    }

    /// 按任务ID查找UI任务
    pub async fn find_ui_task_by_task_id(&self, task_id: &str) -> AppResult<Option<UITask>> {
        let (sql, _params) = SafeQueryBuilder::new("ui_tasks")
            .select(&[
                "ui_id",
                "conversation_id",
                "task_id",
                "name",
                "status",
                "parent_ui_id",
                "render_json",
                "created_at",
                "updated_at",
            ])
            .where_condition(QueryCondition::Eq(
                "task_id".to_string(),
                Value::String(task_id.to_string()),
            ))
            .build()?;

        let row = sqlx::query(&sql)
            .bind(task_id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(UITask::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 创建或更新UI任务（upsert操作）
    pub async fn upsert_ui_task(&self, task: &UITask) -> AppResult<i64> {
        // 使用 INSERT OR REPLACE 语法
        let sql = "INSERT OR REPLACE INTO ui_tasks
                   (ui_id, conversation_id, task_id, name, status, parent_ui_id, render_json, created_at, updated_at)
                   VALUES (
                       (SELECT ui_id FROM ui_tasks WHERE conversation_id = ? AND task_id = ?),
                       ?, ?, ?, ?, ?, ?,
                       COALESCE((SELECT created_at FROM ui_tasks WHERE conversation_id = ? AND task_id = ?), CURRENT_TIMESTAMP),
                       CURRENT_TIMESTAMP
                   )";

        let result = sqlx::query(sql)
            .bind(task.conversation_id)
            .bind(&task.task_id)
            .bind(task.conversation_id)
            .bind(&task.task_id)
            .bind(&task.name)
            .bind(task.status.as_str())
            .bind(task.parent_ui_id)
            .bind(&task.render_json)
            .bind(task.conversation_id)
            .bind(&task.task_id)
            .execute(self.database.pool())
            .await?;

        Ok(result.last_insert_rowid())
    }

    /// 获取UI任务列表（新API名称）
    pub async fn get_ui_tasks(&self, conversation_id: i64) -> AppResult<Vec<UITask>> {
        self.get_ui_tasks_by_conversation(conversation_id).await
    }

    /// 获取任务的最新状态
    pub async fn get_latest_eko_state(&self, task_id: &str) -> AppResult<Option<EkoContext>> {
        let row = sqlx::query(
            "SELECT id, task_id, conversation_id, kind, name, node_id, status, payload_json, created_at 
             FROM eko_context 
             WHERE task_id = ? AND kind = 'state' 
             ORDER BY created_at DESC 
             LIMIT 1"
        )
        .bind(task_id)
        .fetch_optional(self.database.pool())
        .await?;

        match row {
            Some(row) => Ok(Some(EkoContext::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 重建任务执行上下文（用于恢复/重跑）
    pub async fn rebuild_eko_context(
        &self,
        task_id: &str,
        from_snapshot_name: Option<&str>,
    ) -> AppResult<String> {
        // 如果指定了快照名称，从快照开始重建
        if let Some(snapshot_name) = from_snapshot_name {
            let snapshot_row = sqlx::query(
                "SELECT payload_json FROM eko_context 
                 WHERE task_id = ? AND kind = 'snapshot' AND name = ? 
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(task_id)
            .bind(snapshot_name)
            .fetch_optional(self.database.pool())
            .await?;

            if let Some(row) = snapshot_row {
                let payload: String = row.try_get("payload_json")?;
                return Ok(payload);
            }
        }

        // 否则从最新状态重建
        if let Some(state) = self.get_latest_eko_state(task_id).await? {
            return Ok(state.payload_json);
        }

        // 如果没有状态，从所有事件重建
        let events = sqlx::query(
            "SELECT payload_json FROM eko_context 
             WHERE task_id = ? AND kind = 'event' 
             ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(self.database.pool())
        .await?;

        let mut context = serde_json::json!({
            "task_id": task_id,
            "events": []
        });

        for row in events {
            let payload: String = row.try_get("payload_json")?;
            if let Ok(event_json) = serde_json::from_str::<serde_json::Value>(&payload) {
                context["events"].as_array_mut().unwrap().push(event_json);
            }
        }

        Ok(context.to_string())
    }

    /// 构建Prompt（统一入口）- 基于eko原始数据
    pub async fn build_prompt(
        &self,
        task_id: &str,
        user_input: &str,
        _pane_id: Option<&str>,
        _tag_context: Option<&str>,
    ) -> AppResult<String> {
        // 1. 从eko_context获取完整的历史上下文
        let eko_history = self.get_eko_full_history(task_id).await?;

        if !eko_history.is_empty() {
            // 2. 如果有eko历史数据，直接使用（保证不失真）
            return Ok(format!("{}\n\n## 当前问题\n{}", eko_history, user_input));
        }

        // 3. 如果eko数据为空（第一次），使用enhanced_context构建完整上下文
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/workspace".to_string());

        let context_manager = crate::ai::enhanced_context::create_context_manager();
        let workspace_context = context_manager.build_workspace_context(&cwd).await;

        let mut parts = Vec::new();
        parts.push(workspace_context);

        parts.push(format!("## 当前问题\n{}", user_input));

        Ok(parts.join("\n\n"))
    }

    /// 获取eko完整历史上下文（包含所有对话历史）
    async fn get_eko_full_history(&self, task_id: &str) -> AppResult<String> {
        // 1. 获取最新的state记录（包含完整上下文）
        if let Some(state) = self.get_latest_eko_state(task_id).await? {
            // 解析state中的payload_json，提取完整的历史上下文
            if let Ok(state_data) = serde_json::from_str::<serde_json::Value>(&state.payload_json) {
                return self.extract_full_context_from_state(&state_data);
            }
            // 如果解析失败，直接返回原始数据
            return Ok(state.payload_json);
        }

        // 2. 如果没有state，从所有events按时间顺序重建完整历史
        let events = sqlx::query(
            "SELECT payload_json, created_at FROM eko_context 
             WHERE task_id = ? AND kind = 'event' 
             ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(self.database.pool())
        .await?;

        if !events.is_empty() {
            return self.rebuild_history_from_events(&events).await;
        }

        // 3. 没有任何eko数据
        Ok(String::new())
    }

    /// 从state数据中提取完整上下文
    fn extract_full_context_from_state(&self, state_data: &serde_json::Value) -> AppResult<String> {
        let mut context_parts = Vec::new();

        // 1. 提取系统提示词/工作区信息
        if let Some(system_prompt) = state_data.get("systemPrompt").and_then(|p| p.as_str()) {
            if !system_prompt.trim().is_empty() {
                context_parts.push(system_prompt.to_string());
            }
        }

        // 2. 提取任务描述
        if let Some(task_prompt) = state_data.get("taskPrompt").and_then(|p| p.as_str()) {
            if !task_prompt.trim().is_empty() {
                context_parts.push(format!("## 任务描述\n{}", task_prompt));
            }
        }

        // 3. 提取对话历史
        if let Some(history) = state_data
            .get("conversationHistory")
            .and_then(|h| h.as_array())
        {
            if !history.is_empty() {
                let mut history_parts = Vec::new();
                for msg in history {
                    if let (Some(role), Some(content)) = (
                        msg.get("role").and_then(|r| r.as_str()),
                        msg.get("content").and_then(|c| c.as_str()),
                    ) {
                        history_parts.push(format!("{}: {}", role, content));
                    }
                }
                if !history_parts.is_empty() {
                    context_parts.push(format!("## 对话历史\n{}", history_parts.join("\n")));
                }
            }
        }

        // 4. 提取任务结构/步骤
        if let Some(xml) = state_data.get("xml").and_then(|x| x.as_str()) {
            if xml != "<task></task>" && !xml.trim().is_empty() {
                context_parts.push(format!("## 任务结构\n{}", xml));
            }
        }

        if let Some(nodes) = state_data.get("nodes").and_then(|n| n.as_array()) {
            if !nodes.is_empty() {
                let node_texts: Vec<String> = nodes
                    .iter()
                    .filter_map(|node| node.get("text").and_then(|t| t.as_str()))
                    .map(|text| format!("- {}", text))
                    .collect();

                if !node_texts.is_empty() {
                    context_parts.push(format!("## 执行步骤\n{}", node_texts.join("\n")));
                }
            }
        }

        Ok(context_parts.join("\n\n"))
    }

    /// 从事件序列重建完整历史
    async fn rebuild_history_from_events(
        &self,
        events: &[sqlx::sqlite::SqliteRow],
    ) -> AppResult<String> {
        let mut context_parts = Vec::new();
        let mut conversation_history = Vec::new();
        let mut task_info: Option<String> = None;
        let mut system_prompt: Option<String> = None;

        for row in events {
            let payload: String = row.try_get("payload_json")?;
            if let Ok(event_json) = serde_json::from_str::<serde_json::Value>(&payload) {
                let event_type = event_json
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                match event_type {
                    "agent_start" => {
                        // 提取任务信息
                        if let Some(task) = event_json.get("task") {
                            task_info = self.build_context_from_task(task);
                        }
                        // 提取系统提示词
                        if let Some(prompt) =
                            event_json.get("systemPrompt").and_then(|p| p.as_str())
                        {
                            system_prompt = Some(prompt.to_string());
                        }
                    }
                    "user_message" => {
                        // 用户消息
                        if let Some(content) = event_json.get("content").and_then(|c| c.as_str()) {
                            conversation_history.push(format!("用户: {}", content));
                        }
                    }
                    "assistant_message" => {
                        // 助手消息
                        if let Some(content) = event_json.get("content").and_then(|c| c.as_str()) {
                            conversation_history.push(format!("助手: {}", content));
                        }
                    }
                    "thinking" => {
                        // 思考过程
                        if let Some(text) = event_json.get("text").and_then(|t| t.as_str()) {
                            conversation_history.push(format!("思考: {}", text));
                        }
                    }
                    _ => {
                        // 其他事件，尝试提取有用信息
                        if let Some(context_info) = self.extract_context_from_event(&event_json) {
                            if !context_parts.contains(&context_info) {
                                context_parts.push(context_info);
                            }
                        }
                    }
                }
            }
        }

        // 组装完整上下文
        let mut final_parts = Vec::new();

        // 1. 系统提示词
        if let Some(prompt) = system_prompt {
            final_parts.push(prompt);
        }

        // 2. 任务信息
        if let Some(task) = task_info {
            final_parts.push(task);
        }

        // 3. 对话历史
        if !conversation_history.is_empty() {
            final_parts.push(format!("## 对话历史\n{}", conversation_history.join("\n")));
        }

        // 4. 其他上下文
        final_parts.extend(context_parts);

        Ok(final_parts.join("\n\n"))
    }

    /// 从事件中提取上下文信息
    fn extract_context_from_event(&self, event: &serde_json::Value) -> Option<String> {
        // 1. 优先处理agent_context类型的state数据
        if event.get("type").and_then(|t| t.as_str()) == Some("agent_context") {
            return self.build_context_from_agent_state(event);
        }

        // 2. 处理agent_start事件
        if event.get("type").and_then(|t| t.as_str()) == Some("agent_start") {
            if let Some(task) = event.get("task") {
                return self.build_context_from_task(task);
            }
        }

        // 3. 处理其他包含task信息的事件
        if let Some(task) = event.get("task") {
            return self.build_context_from_task(task);
        }

        None
    }

    /// 从agent_context状态构建上下文
    fn build_context_from_agent_state(&self, state: &serde_json::Value) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(task_prompt) = state.get("taskPrompt").and_then(|p| p.as_str()) {
            if !task_prompt.trim().is_empty() {
                parts.push(format!("## 任务描述\n{}", task_prompt));
            }
        }

        if let Some(xml) = state.get("xml").and_then(|x| x.as_str()) {
            if xml != "<task></task>" && !xml.trim().is_empty() {
                parts.push(format!("## 任务结构\n{}", xml));
            }
        }

        if let Some(nodes) = state.get("nodes").and_then(|n| n.as_array()) {
            if !nodes.is_empty() {
                let node_texts: Vec<String> = nodes
                    .iter()
                    .filter_map(|node| node.get("text").and_then(|t| t.as_str()))
                    .map(|text| format!("- {}", text))
                    .collect();

                if !node_texts.is_empty() {
                    parts.push(format!("## 执行步骤\n{}", node_texts.join("\n")));
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    /// 从task对象构建上下文
    fn build_context_from_task(&self, task: &serde_json::Value) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(task_prompt) = task.get("taskPrompt").and_then(|p| p.as_str()) {
            if !task_prompt.trim().is_empty() {
                parts.push(format!("## 任务描述\n{}", task_prompt));
            }
        }

        if let Some(description) = task.get("description").and_then(|d| d.as_str()) {
            if !description.trim().is_empty()
                && description
                    != task
                        .get("taskPrompt")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
            {
                parts.push(format!("## 详细说明\n{}", description));
            }
        }

        if let Some(xml) = task.get("xml").and_then(|x| x.as_str()) {
            if xml != "<task></task>" && !xml.trim().is_empty() {
                parts.push(format!("## 任务结构\n{}", xml));
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }


    /// 获取快照列表
    pub async fn get_snapshots(&self, task_id: &str) -> AppResult<Vec<EkoContext>> {
        let rows = sqlx::query(
            "SELECT id, task_id, conversation_id, kind, name, node_id, status, payload_json, created_at 
             FROM eko_context 
             WHERE task_id = ? AND kind = 'snapshot' 
             ORDER BY created_at DESC"
        )
        .bind(task_id)
        .fetch_all(self.database.pool())
        .await?;

        let snapshots: Vec<EkoContext> = rows
            .iter()
            .map(|row| EkoContext::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(snapshots)
    }
}

#[async_trait::async_trait]
impl Repository<UITask> for TaskRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<UITask>> {
        let (sql, _params) = SafeQueryBuilder::new("ui_tasks")
            .select(&[
                "ui_id",
                "conversation_id",
                "task_id",
                "name",
                "status",
                "parent_ui_id",
                "render_json",
                "created_at",
                "updated_at",
            ])
            .where_condition(QueryCondition::Eq(
                "ui_id".to_string(),
                Value::Number(id.into()),
            ))
            .build()?;

        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(UITask::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<UITask>> {
        let (sql, _params) = SafeQueryBuilder::new("ui_tasks")
            .select(&[
                "ui_id",
                "conversation_id",
                "task_id",
                "name",
                "status",
                "parent_ui_id",
                "render_json",
                "created_at",
                "updated_at",
            ])
            .order_by(crate::storage::query::QueryOrder::Desc(
                "updated_at".to_string(),
            ))
            .build()?;

        let rows = sqlx::query(&sql).fetch_all(self.database.pool()).await?;

        let tasks: Vec<UITask> = rows
            .iter()
            .map(|row| UITask::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    async fn save(&self, entity: &UITask) -> AppResult<i64> {
        self.create_ui_task(entity).await
    }

    async fn update(&self, entity: &UITask) -> AppResult<()> {
        if let Some(ui_id) = entity.ui_id {
            self.update_ui_task(ui_id, entity).await
        } else {
            Err(anyhow!("无法更新没有ID的UI任务"))
        }
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        self.delete_ui_task(id).await
    }
}
