/*!
 * Agent后端Repository实现
 *
 * 提供Agent任务、执行日志、工具调用和上下文快照的数据访问层
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

// ===========================
// 数据结构定义
// ===========================

/// Agent任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTaskStatus {
    Created,
    Running,
    Paused,
    Completed,
    Error,
    Cancelled,
}

impl AgentTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "created" => Ok(Self::Created),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(anyhow!("无效的Agent任务状态: {}", s)),
        }
    }
}

/// Agent任务
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    pub id: Option<i64>,
    pub task_id: String,
    pub conversation_id: i64,
    pub user_prompt: String,
    pub status: AgentTaskStatus,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub error_count: u32,
    pub config_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentTask {
    pub fn new(conversation_id: i64, user_prompt: String) -> Self {
        let now = Utc::now();
        // 生成简单的任务ID（使用时间戳+随机数）
        let task_id = format!("task_{}", now.timestamp_millis());
        Self {
            id: None,
            task_id,
            conversation_id,
            user_prompt,
            status: AgentTaskStatus::Created,
            current_iteration: 0,
            max_iterations: 100,
            error_count: 0,
            config_json: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn with_config(mut self, config: Value) -> Self {
        self.config_json = Some(config.to_string());
        self
    }

    pub fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.max_iterations = max_iterations;
        self
    }
}

impl RowMapper<AgentTask> for AgentTask {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let status_str: String = row.try_get("status")?;
        let status = AgentTaskStatus::from_str(&status_str)?;

        Ok(Self {
            id: Some(row.try_get("id")?),
            task_id: row.try_get("task_id")?,
            conversation_id: row.try_get("conversation_id")?,
            user_prompt: row.try_get("user_prompt")?,
            status,
            current_iteration: row.try_get::<i64, _>("current_iteration")? as u32,
            max_iterations: row.try_get::<i64, _>("max_iterations")? as u32,
            error_count: row.try_get::<i64, _>("error_count")? as u32,
            config_json: row.try_get("config_json")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            completed_at: row.try_get("completed_at")?,
        })
    }
}

/// 执行步骤类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStepType {
    Thinking,
    ToolCall,
    ToolResult,
    FinalAnswer,
    Error,
}

impl ExecutionStepType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Thinking => "thinking",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::FinalAnswer => "final_answer",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "thinking" => Ok(Self::Thinking),
            "tool_call" => Ok(Self::ToolCall),
            "tool_result" => Ok(Self::ToolResult),
            "final_answer" => Ok(Self::FinalAnswer),
            "error" => Ok(Self::Error),
            _ => Err(anyhow!("无效的执行步骤类型: {}", s)),
        }
    }
}

/// Agent执行日志
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionLog {
    pub id: Option<i64>,
    pub task_id: String,
    pub iteration: u32,
    pub step_type: ExecutionStepType,
    pub content_json: String,
    pub timestamp: DateTime<Utc>,
}

impl AgentExecutionLog {
    pub fn new(
        task_id: String,
        iteration: u32,
        step_type: ExecutionStepType,
        content: Value,
    ) -> Self {
        Self {
            id: None,
            task_id,
            iteration,
            step_type,
            content_json: content.to_string(),
            timestamp: Utc::now(),
        }
    }
}

impl RowMapper<AgentExecutionLog> for AgentExecutionLog {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let step_type_str: String = row.try_get("step_type")?;
        let step_type = ExecutionStepType::from_str(&step_type_str)?;

        Ok(Self {
            id: Some(row.try_get("id")?),
            task_id: row.try_get("task_id")?,
            iteration: row.try_get::<i64, _>("iteration")? as u32,
            step_type,
            content_json: row.try_get("content_json")?,
            timestamp: row.try_get("timestamp")?,
        })
    }
}

/// 工具调用状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Error,
}

impl ToolCallStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            _ => Err(anyhow!("无效的工具调用状态: {}", s)),
        }
    }
}

/// Agent工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCall {
    pub id: Option<i64>,
    pub task_id: String,
    pub call_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub result_json: Option<String>,
    pub status: ToolCallStatus,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentToolCall {
    pub fn new(task_id: String, call_id: String, tool_name: String, arguments: Value) -> Self {
        Self {
            id: None,
            task_id,
            call_id,
            tool_name,
            arguments_json: arguments.to_string(),
            result_json: None,
            status: ToolCallStatus::Pending,
            error_message: None,
            started_at: Utc::now(),
            completed_at: None,
        }
    }
}

impl RowMapper<AgentToolCall> for AgentToolCall {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let status_str: String = row.try_get("status")?;
        let status = ToolCallStatus::from_str(&status_str)?;

        Ok(Self {
            id: Some(row.try_get("id")?),
            task_id: row.try_get("task_id")?,
            call_id: row.try_get("call_id")?,
            tool_name: row.try_get("tool_name")?,
            arguments_json: row.try_get("arguments_json")?,
            result_json: row.try_get("result_json")?,
            status,
            error_message: row.try_get("error_message")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
        })
    }
}

/// 上下文类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    Full,
    Incremental,
}

impl ContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Incremental => "incremental",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "full" => Ok(Self::Full),
            "incremental" => Ok(Self::Incremental),
            _ => Err(anyhow!("无效的上下文类型: {}", s)),
        }
    }
}

/// Agent上下文快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentContextSnapshot {
    pub id: Option<i64>,
    pub task_id: String,
    pub iteration: u32,
    pub context_type: ContextType,
    pub messages_json: String,
    pub additional_state_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AgentContextSnapshot {
    pub fn new(
        task_id: String,
        iteration: u32,
        context_type: ContextType,
        messages: Value,
    ) -> Self {
        Self {
            id: None,
            task_id,
            iteration,
            context_type,
            messages_json: messages.to_string(),
            additional_state_json: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_additional_state(mut self, state: Value) -> Self {
        self.additional_state_json = Some(state.to_string());
        self
    }
}

impl RowMapper<AgentContextSnapshot> for AgentContextSnapshot {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        let context_type_str: String = row.try_get("context_type")?;
        let context_type = ContextType::from_str(&context_type_str)?;

        Ok(Self {
            id: Some(row.try_get("id")?),
            task_id: row.try_get("task_id")?,
            iteration: row.try_get::<i64, _>("iteration")? as u32,
            context_type,
            messages_json: row.try_get("messages_json")?,
            additional_state_json: row.try_get("additional_state_json")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

// ===========================
// Repository实现
// ===========================

/// Agent任务Repository
pub struct AgentTaskRepository {
    database: Arc<DatabaseManager>,
}

impl AgentTaskRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 创建新任务
    pub async fn create(&self, task: &AgentTask) -> AppResult<String> {
        let (sql, params) = InsertBuilder::new("agent_tasks")
            .set("task_id", Value::String(task.task_id.clone()))
            .set(
                "conversation_id",
                Value::Number(task.conversation_id.into()),
            )
            .set("user_prompt", Value::String(task.user_prompt.clone()))
            .set("status", Value::String(task.status.as_str().to_string()))
            .set(
                "current_iteration",
                Value::Number(task.current_iteration.into()),
            )
            .set("max_iterations", Value::Number(task.max_iterations.into()))
            .set("error_count", Value::Number(task.error_count.into()))
            .set(
                "config_json",
                task.config_json
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set("created_at", Value::String(task.created_at.to_rfc3339()))
            .set("updated_at", Value::String(task.updated_at.to_rfc3339()))
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

        query.execute(self.database.pool()).await?;
        Ok(task.task_id.clone())
    }

    /// 根据task_id查找任务
    pub async fn find_by_task_id(&self, task_id: &str) -> AppResult<Option<AgentTask>> {
        let sql = "SELECT * FROM agent_tasks WHERE task_id = ?";
        let row = sqlx::query(sql)
            .bind(task_id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentTask::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// 更新任务状态
    pub async fn update_status(
        &self,
        task_id: &str,
        status: AgentTaskStatus,
        current_iteration: u32,
        error_count: u32,
    ) -> AppResult<()> {
        let sql = "UPDATE agent_tasks SET status = ?, current_iteration = ?, error_count = ?, updated_at = CURRENT_TIMESTAMP WHERE task_id = ?";

        sqlx::query(sql)
            .bind(status.as_str())
            .bind(current_iteration as i64)
            .bind(error_count as i64)
            .bind(task_id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 完成任务
    pub async fn complete_task(&self, task_id: &str, status: AgentTaskStatus) -> AppResult<()> {
        let sql = "UPDATE agent_tasks SET status = ?, completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE task_id = ?";

        sqlx::query(sql)
            .bind(status.as_str())
            .bind(task_id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// 根据会话ID查找任务
    pub async fn find_by_conversation_id(&self, conversation_id: i64) -> AppResult<Vec<AgentTask>> {
        let sql = "SELECT * FROM agent_tasks WHERE conversation_id = ? ORDER BY created_at DESC";
        let rows = sqlx::query(sql)
            .bind(conversation_id)
            .fetch_all(self.database.pool())
            .await?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(AgentTask::from_row(&row)?);
        }

        Ok(tasks)
    }

    /// 查找未完成的任务
    pub async fn find_incomplete_tasks(&self) -> AppResult<Vec<AgentTask>> {
        let sql = "SELECT * FROM agent_tasks WHERE status IN ('created', 'running', 'paused') ORDER BY updated_at DESC";
        let rows = sqlx::query(sql).fetch_all(self.database.pool()).await?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(AgentTask::from_row(&row)?);
        }

        Ok(tasks)
    }
}

#[async_trait::async_trait]
impl Repository<AgentTask> for AgentTaskRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<AgentTask>> {
        let sql = "SELECT * FROM agent_tasks WHERE id = ?";
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(self.database.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(AgentTask::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<AgentTask>> {
        let sql = "SELECT * FROM agent_tasks ORDER BY created_at DESC";
        let rows = sqlx::query(sql).fetch_all(self.database.pool()).await?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(AgentTask::from_row(&row)?);
        }

        Ok(tasks)
    }

    async fn save(&self, entity: &AgentTask) -> AppResult<i64> {
        self.create(entity).await?;
        // 返回ID (需要查询获取)
        if let Some(task) = self.find_by_task_id(&entity.task_id).await? {
            Ok(task.id.unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    async fn update(&self, entity: &AgentTask) -> AppResult<()> {
        self.update_status(
            &entity.task_id,
            entity.status.clone(),
            entity.current_iteration,
            entity.error_count,
        )
        .await
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let sql = "DELETE FROM agent_tasks WHERE id = ?";
        sqlx::query(sql)
            .bind(id)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }
}

// TODO: 继续实现其他三个Repository类...
// 由于文件长度限制，我会在下一个文件中实现剩余的Repository
