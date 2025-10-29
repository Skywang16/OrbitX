/*!
 * 审计日志访问层 - 直接使用 sqlx
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

impl AuditLogEntry {
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

/// 审计日志访问器
pub struct AuditLogs<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AuditLogs<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
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
        let mut sql = String::from(
            "SELECT id, operation, table_name, record_id, user_context, details, timestamp, success, error_message FROM audit_logs",
        );
        let mut where_clauses: Vec<&str> = Vec::new();

        // Collect conditions
        if table_name.is_some() {
            where_clauses.push("table_name = ?");
        }
        if operation.is_some() {
            where_clauses.push("operation = ?");
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY timestamp DESC");
        if limit.is_some() {
            sql.push_str(" LIMIT ?");
        }

        let mut qb = sqlx::query(&sql);
        if let Some(table) = table_name {
            qb = qb.bind(table);
        }
        if let Some(op) = operation {
            qb = qb.bind(op);
        }
        if let Some(limit) = limit {
            qb = qb.bind(limit);
        }

        let rows = qb.fetch_all(self.db.pool()).await?;
        let entries: Vec<AuditLogEntry> = rows
            .iter()
            .map(|row| AuditLogEntry::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
    pub async fn find_by_id(&self, id: i64) -> RepositoryResult<Option<AuditLogEntry>> {
        let row = sqlx::query(
            r#"SELECT id, operation, table_name, record_id, user_context, details, timestamp, success, error_message
            FROM audit_logs WHERE id = ? LIMIT 1"#,
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| AuditLogEntry::from_row(&r)).transpose()?)
    }

    pub async fn find_all(&self) -> RepositoryResult<Vec<AuditLogEntry>> {
        self.find_logs(None, None, None).await
    }

    pub async fn save(&self, entity: &AuditLogEntry) -> RepositoryResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO audit_logs (operation, table_name, record_id, user_context, details, success, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&entity.operation)
        .bind(&entity.table_name)
        .bind(&entity.record_id)
        .bind(&entity.user_context)
        .bind(&entity.details)
        .bind(entity.success)
        .bind(&entity.error_message)
        .execute(self.db.pool())
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete(&self, id: i64) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM audit_logs WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::AuditLogNotFound { id: id.to_string() });
        }
        Ok(())
    }
}
