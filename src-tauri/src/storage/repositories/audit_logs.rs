/*!
 * 审计日志Repository
 *
 * 处理审计日志的数据访问逻辑
 */

use super::{Repository, RowMapper};
use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use crate::storage::query::{InsertBuilder, QueryCondition, SafeQueryBuilder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Option<i64>,
    pub operation: String,
    pub table_name: String,
    pub record_id: Option<String>,
    pub user_context: Option<String>,
    pub details: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl AuditLogEntry {
    pub fn new(
        operation: String,
        table_name: String,
        record_id: Option<String>,
        user_context: Option<String>,
        details: String,
        success: bool,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id: None,
            operation,
            table_name,
            record_id,
            user_context,
            details,
            timestamp: Utc::now(),
            success,
            error_message,
        }
    }
}

impl RowMapper<AuditLogEntry> for AuditLogEntry {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> RepositoryResult<Self> {
        Ok(Self {
            id: Some(row.try_get("id")?),
            operation: row.try_get("operation")?,
            table_name: row.try_get("table_name")?,
            record_id: row.try_get("record_id")?,
            user_context: row.try_get("user_context")?,
            details: row.try_get("details")?,
            timestamp: row.try_get("timestamp")?,
            success: row.try_get("success")?,
            error_message: row.try_get("error_message")?,
        })
    }
}

/// 审计日志Repository
pub struct AuditLogRepository {
    database: Arc<DatabaseManager>,
}

impl AuditLogRepository {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// 记录审计事件
    pub async fn log_event(
        &self,
        operation: &str,
        table_name: &str,
        record_id: Option<&str>,
        user_context: Option<&str>,
        details: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> RepositoryResult<i64> {
        let entry = AuditLogEntry::new(
            operation.to_string(),
            table_name.to_string(),
            record_id.map(|s| s.to_string()),
            user_context.map(|s| s.to_string()),
            details.to_string(),
            success,
            error_message.map(|s| s.to_string()),
        );

        self.save(&entry).await
    }

    /// 查询审计日志
    pub async fn find_logs(
        &self,
        table_name: Option<&str>,
        operation: Option<&str>,
        limit: Option<i64>,
    ) -> RepositoryResult<Vec<AuditLogEntry>> {
        let mut builder = SafeQueryBuilder::new("audit_logs")
            .select(&[
                "id",
                "operation",
                "table_name",
                "record_id",
                "user_context",
                "details",
                "timestamp",
                "success",
                "error_message",
            ])
            .order_by(crate::storage::query::QueryOrder::Desc(
                "timestamp".to_string(),
            ));

        if let Some(table) = table_name {
            builder = builder.where_condition(QueryCondition::Eq(
                "table_name".to_string(),
                Value::String(table.to_string()),
            ));
        }

        if let Some(op) = operation {
            builder = builder.where_condition(QueryCondition::Eq(
                "operation".to_string(),
                Value::String(op.to_string()),
            ));
        }

        if let Some(limit) = limit {
            builder = builder.limit(limit);
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
                        return Err(RepositoryError::UnsupportedNumberType);
                    }
                }
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "audit_logs parameter",
                    ))
                }
            };
        }

        let rows = query_builder.fetch_all(self.database.pool()).await?;
        let entries: Vec<AuditLogEntry> = rows
            .iter()
            .map(|row| AuditLogEntry::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
}

#[async_trait::async_trait]
impl Repository<AuditLogEntry> for AuditLogRepository {
    async fn find_by_id(&self, id: i64) -> RepositoryResult<Option<AuditLogEntry>> {
        let (sql, _params) = SafeQueryBuilder::new("audit_logs")
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
            Some(row) => Ok(Some(AuditLogEntry::from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> RepositoryResult<Vec<AuditLogEntry>> {
        self.find_logs(None, None, None).await
    }

    async fn save(&self, entity: &AuditLogEntry) -> RepositoryResult<i64> {
        let (sql, params) = InsertBuilder::new("audit_logs")
            .set("operation", Value::String(entity.operation.clone()))
            .set("table_name", Value::String(entity.table_name.clone()))
            .set(
                "record_id",
                entity
                    .record_id
                    .as_ref()
                    .map(|r| Value::String(r.clone()))
                    .unwrap_or(Value::Null),
            )
            .set(
                "user_context",
                entity
                    .user_context
                    .as_ref()
                    .map(|u| Value::String(u.clone()))
                    .unwrap_or(Value::Null),
            )
            .set("details", Value::String(entity.details.clone()))
            .set("success", Value::Bool(entity.success))
            .set(
                "error_message",
                entity
                    .error_message
                    .as_ref()
                    .map(|e| Value::String(e.clone()))
                    .unwrap_or(Value::Null),
            )
            .build()?;

        let mut query_builder = sqlx::query(&sql);
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Bool(b) => query_builder.bind(b),
                Value::Null => query_builder.bind(None::<String>),
                _ => {
                    return Err(RepositoryError::unsupported_parameter(
                        "audit_logs parameter",
                    ))
                }
            };
        }

        let result = query_builder.execute(self.database.pool()).await?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, _entity: &AuditLogEntry) -> RepositoryResult<()> {
        Err(RepositoryError::AuditLogUpdateNotSupported)
    }

    async fn delete(&self, id: i64) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM audit_logs WHERE id = ?")
            .bind(id)
            .execute(self.database.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::AuditLogNotFound {
                id: id.to_string(),
            });
        }

        Ok(())
    }
}
