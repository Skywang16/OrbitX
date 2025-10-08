/*!
 * 命令历史Repository
 *
 * 处理命令历史记录的数据访问逻辑
 */

use super::{Ordering, Pagination, Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use tracing::info;

/// 命令历史条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub id: Option<i64>,
    pub command: String,
    pub working_directory: String,
    pub exit_code: Option<i32>,
    pub output: Option<String>,
    pub duration_ms: Option<i64>,
    pub executed_at: DateTime<Utc>,
    pub session_id: Option<String>,
    pub tags: Option<String>,
}

impl CommandHistoryEntry {
    pub fn new(
        command: String,
        working_directory: String,
        exit_code: Option<i32>,
        output: Option<String>,
        duration_ms: Option<i64>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            id: None,
            command,
            working_directory,
            exit_code,
            output,
            duration_ms,
            executed_at: Utc::now(),
            session_id,
            tags: None,
        }
    }
}

impl RowMapper<CommandHistoryEntry> for CommandHistoryEntry {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Self> {
        Ok(Self {
            id: Some(row.try_get("id")?),
            command: row.try_get("command")?,
            working_directory: row.try_get("working_directory")?,
            exit_code: row.try_get("exit_code")?,
            output: row.try_get("output")?,
            duration_ms: row.try_get("duration_ms")?,
            executed_at: row.try_get("executed_at")?,
            session_id: row.try_get("session_id")?,
            tags: row.try_get("tags")?,
        })
    }
}

/// 历史查询条件
#[derive(Debug, Clone)]
pub struct HistoryQuery {
    pub command_pattern: Option<String>,
    pub working_directory: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub pagination: Option<Pagination>,
    pub ordering: Option<Ordering>,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            command_pattern: None,
            working_directory: None,
            date_from: None,
            date_to: None,
            session_id: None,
            pagination: None,
            ordering: Some(Ordering::desc("executed_at")),
        }
    }
}

/// 命令搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSearchResult {
    pub id: i64,
    pub command: String,
    pub working_directory: String,
    pub output: Option<String>,
    pub executed_at: DateTime<Utc>,
    pub command_snippet: Option<String>,
    pub output_snippet: Option<String>,
    pub relevance_score: f64,
}

/// 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_commands: i64,
    pub unique_commands: i64,
    pub avg_execution_time: f64,
    pub most_used_commands: Vec<(String, i64)>,
    pub recent_activity: Vec<CommandHistoryEntry>,
}

/// 命令历史Repository
pub struct CommandHistoryRepository {
    database: Arc<DatabaseManager>,
}

impl CommandHistoryRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 根据查询条件查找命令历史
    pub async fn find_by_query(&self, query: &HistoryQuery) -> AppResult<Vec<CommandHistoryEntry>> {
        let mut builder = SafeQueryBuilder::new("command_history").select(&[
            "id",
            "command",
            "working_directory",
            "exit_code",
            "output",
            "duration_ms",
            "executed_at",
            "session_id",
            "tags",
        ]);

        // 添加查询条件
        if let Some(pattern) = &query.command_pattern {
            builder = builder.where_condition(QueryCondition::Like(
                "command".to_string(),
                format!("%{}%", pattern),
            ));
        }

        if let Some(working_dir) = &query.working_directory {
            builder = builder.where_condition(QueryCondition::Eq(
                "working_directory".to_string(),
                Value::String(working_dir.clone()),
            ));
        }

        if let Some(session_id) = &query.session_id {
            builder = builder.where_condition(QueryCondition::Eq(
                "session_id".to_string(),
                Value::String(session_id.clone()),
            ));
        }

        if let Some(date_from) = &query.date_from {
            builder = builder.where_condition(QueryCondition::Ge(
                "executed_at".to_string(),
                Value::String(date_from.to_rfc3339()),
            ));
        }

        if let Some(date_to) = &query.date_to {
            builder = builder.where_condition(QueryCondition::Le(
                "executed_at".to_string(),
                Value::String(date_to.to_rfc3339()),
            ));
        }

        // 添加排序
        if let Some(ordering) = &query.ordering {
            builder = builder.order_by(match ordering.desc {
                true => crate::storage::query::QueryOrder::Desc(ordering.field.clone()),
                false => crate::storage::query::QueryOrder::Asc(ordering.field.clone()),
            });
        }

        // 添加分页
        if let Some(pagination) = &query.pagination {
            if let Some(limit) = pagination.limit {
                builder = builder.limit(limit);
            }
            if let Some(offset) = pagination.offset {
                builder = builder.offset(offset);
            }
        }

        let (sql, params) = builder.build()?;

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

        let rows = query_builder.fetch_all(self.database.pool()).await?;
        let entries: Vec<CommandHistoryEntry> = rows
            .iter()
            .map(|row| CommandHistoryEntry::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// 全文搜索命令历史
    pub async fn full_text_search(
        &self,
        search_query: &str,
    ) -> AppResult<Vec<CommandSearchResult>> {
        let sql = r#"
            SELECT ch.id, ch.command, ch.working_directory, ch.output, ch.executed_at,
                   snippet(command_search, 0, '<mark>', '</mark>', '...', 32) as command_snippet,
                   snippet(command_search, 1, '<mark>', '</mark>', '...', 64) as output_snippet
            FROM command_search
            JOIN command_history ch ON command_search.rowid = ch.id
            WHERE command_search MATCH ?
            ORDER BY rank
            LIMIT 50
        "#;

        let rows = sqlx::query(sql)
            .bind(search_query)
            .fetch_all(self.database.pool())
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let result = CommandSearchResult {
                id: row.try_get("id")?,
                command: row.try_get("command")?,
                working_directory: row.try_get("working_directory")?,
                output: row.try_get("output")?,
                executed_at: row.try_get("executed_at")?,
                command_snippet: row.try_get("command_snippet")?,
                output_snippet: row.try_get("output_snippet")?,
                relevance_score: 1.0, // FTS5会自动排序
            };
            results.push(result);
        }

        Ok(results)
    }

    /// 获取使用统计
    pub async fn get_usage_statistics(&self) -> AppResult<UsageStats> {
        // 总命令数
        let total_commands: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_history")
            .fetch_one(self.database.pool())
            .await?;

        // 唯一命令数
        let unique_commands: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT command) FROM command_history")
                .fetch_one(self.database.pool())
                .await?;

        // 平均执行时间
        let avg_execution_time: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(duration_ms) FROM command_history WHERE duration_ms IS NOT NULL",
        )
        .fetch_one(self.database.pool())
        .await?;

        // 最常用命令
        let most_used_rows = sqlx::query(
            r#"
            SELECT command, COUNT(*) as usage_count
            FROM command_history
            GROUP BY command
            ORDER BY usage_count DESC
            LIMIT 10
        "#,
        )
        .fetch_all(self.database.pool())
        .await?;

        let most_used_commands: Vec<(String, i64)> = most_used_rows
            .into_iter()
            .map(|row| Ok((row.try_get("command")?, row.try_get("usage_count")?)))
            .collect::<Result<Vec<_>, sqlx::Error>>()?;

        // 最近活动
        let recent_query = HistoryQuery {
            pagination: Some(Pagination::new(Some(20), None)),
            ..Default::default()
        };
        let recent_activity = self.find_by_query(&recent_query).await?;

        Ok(UsageStats {
            total_commands,
            unique_commands,
            avg_execution_time: avg_execution_time.unwrap_or(0.0),
            most_used_commands,
            recent_activity,
        })
    }

    /// 批量保存命令历史
    pub async fn batch_save(&self, entries: &[CommandHistoryEntry]) -> AppResult<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.database.pool().begin().await?;

        for entry in entries {
            let sql = r#"
                INSERT INTO command_history
                (command, working_directory, exit_code, output, duration_ms, executed_at, session_id, tags)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#;

            sqlx::query(sql)
                .bind(&entry.command)
                .bind(&entry.working_directory)
                .bind(entry.exit_code)
                .bind(&entry.output)
                .bind(entry.duration_ms)
                .bind(entry.executed_at)
                .bind(&entry.session_id)
                .bind(&entry.tags)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        info!("批量保存了 {} 条命令历史记录", entries.len());
        Ok(())
    }

    /// 更新命令使用统计
    pub async fn update_usage_stats(&self, entry: &CommandHistoryEntry) -> AppResult<()> {
        let command_hash = format!("{:x}", md5::compute(&entry.command));

        let sql = r#"
            INSERT INTO command_usage_stats (command_hash, command, working_directory, usage_count, last_used, avg_duration_ms)
            VALUES (?, ?, ?, 1, ?, ?)
            ON CONFLICT(command_hash, working_directory) DO UPDATE SET
                usage_count = usage_count + 1,
                last_used = excluded.last_used,
                avg_duration_ms = (avg_duration_ms * (usage_count - 1) + excluded.avg_duration_ms) / usage_count
        "#;

        sqlx::query(sql)
            .bind(&command_hash)
            .bind(&entry.command)
            .bind(&entry.working_directory)
            .bind(entry.executed_at)
            .bind(entry.duration_ms.unwrap_or(0))
            .execute(self.database.pool())
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Repository<CommandHistoryEntry> for CommandHistoryRepository {
    async fn find_by_id(&self, id: i64) -> AppResult<Option<CommandHistoryEntry>> {
        let (sql, _params) = SafeQueryBuilder::new("command_history")
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
            Some(row) => Ok(Some(CommandHistoryEntry::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<CommandHistoryEntry>> {
        let query = HistoryQuery::default();
        self.find_by_query(&query).await
    }

    async fn save(&self, entity: &CommandHistoryEntry) -> AppResult<i64> {
        let (sql, params) = InsertBuilder::new("command_history")
            .set("command", Value::String(entity.command.clone()))
            .set(
                "working_directory",
                Value::String(entity.working_directory.clone()),
            )
            .set(
                "exit_code",
                entity
                    .exit_code
                    .map(|c| Value::Number(c.into()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "output",
                entity
                    .output
                    .as_ref()
                    .map(|o| Value::String(o.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "duration_ms",
                entity
                    .duration_ms
                    .map(|d| Value::Number(d.into()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "executed_at",
                Value::String(entity.executed_at.to_rfc3339()),
            )
            .set(
                "session_id",
                entity
                    .session_id
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "tags",
                entity
                    .tags
                    .as_ref()
                    .map(|t| Value::String(t.clone()))
                    .unwrap_or(Value::Null),
            )
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else if let Some(i) = n.as_i64() {
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

        // 更新使用统计
        self.update_usage_stats(entity).await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, _entity: &CommandHistoryEntry) -> AppResult<()> {
        Err(anyhow!("Command history does not support update operations"))
    }

    async fn delete(&self, id: i64) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM command_history WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Command history entry does not exist: {}", id));
        }

        Ok(())
    }
}
