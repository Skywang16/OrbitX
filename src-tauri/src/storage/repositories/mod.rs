/*!
 * Repository模式实现
 *
 * 提供数据访问层的抽象，将业务逻辑与数据库操作分离
 */

pub mod ai_features;
pub mod ai_models;
pub mod audit_logs;
pub mod command_history;
pub mod conversations;

// 重新导出所有Repository
pub use ai_features::AIFeaturesRepository;
pub use ai_models::AIModelRepository;
pub use audit_logs::AuditLogRepository;
pub use command_history::CommandHistoryRepository;
pub use conversations::ConversationRepository;

use crate::storage::database::DatabaseManager;
use crate::utils::error::AppResult;
use base64::Engine;
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

/// Repository基础trait
#[async_trait::async_trait]
pub trait Repository<T> {
    /// 根据ID查找实体
    async fn find_by_id(&self, id: i64) -> AppResult<Option<T>>;

    /// 查找所有实体
    async fn find_all(&self) -> AppResult<Vec<T>>;

    /// 保存实体
    async fn save(&self, entity: &T) -> AppResult<i64>;

    /// 更新实体
    async fn update(&self, entity: &T) -> AppResult<()>;

    /// 删除实体
    async fn delete(&self, id: i64) -> AppResult<()>;
}

/// Repository管理器
pub struct RepositoryManager {
    database: Arc<DatabaseManager>,
    ai_features: AIFeaturesRepository,
    ai_models: AIModelRepository,
    audit_logs: AuditLogRepository,
    command_history: CommandHistoryRepository,
    conversations: ConversationRepository,
}

impl RepositoryManager {
    /// 创建新的Repository管理器
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            ai_features: AIFeaturesRepository::new(Arc::clone(&database)),
            ai_models: AIModelRepository::new(Arc::clone(&database)),
            audit_logs: AuditLogRepository::new(Arc::clone(&database)),
            command_history: CommandHistoryRepository::new(Arc::clone(&database)),
            conversations: ConversationRepository::new(Arc::clone(&database)),
            database,
        }
    }

    /// 获取AI功能配置Repository
    pub fn ai_features(&self) -> &AIFeaturesRepository {
        &self.ai_features
    }

    /// 获取AI模型Repository
    pub fn ai_models(&self) -> &AIModelRepository {
        &self.ai_models
    }

    /// 获取审计日志Repository
    pub fn audit_logs(&self) -> &AuditLogRepository {
        &self.audit_logs
    }

    /// 获取命令历史Repository
    pub fn command_history(&self) -> &CommandHistoryRepository {
        &self.command_history
    }

    /// 获取会话Repository
    pub fn conversations(&self) -> &ConversationRepository {
        &self.conversations
    }


    /// 获取数据库管理器
    pub fn database(&self) -> &DatabaseManager {
        &self.database
    }
}

/// 通用的行转换工具
pub trait RowMapper<T> {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<T>;
}

/// 通用的值提取工具
pub fn extract_value_from_row(
    row: &sqlx::sqlite::SqliteRow,
    column_index: usize,
) -> AppResult<Value> {
    use sqlx::{Column, TypeInfo};

    let column = &row.columns()[column_index];
    let column_type = column.type_info();

    match column_type.name() {
        "TEXT" => {
            let value: Option<String> = row.try_get(column_index)?;
            Ok(value.map(Value::String).unwrap_or(Value::Null))
        }
        "INTEGER" => {
            let value: Option<i64> = row.try_get(column_index)?;
            Ok(value
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null))
        }
        "REAL" => {
            let value: Option<f64> = row.try_get(column_index)?;
            Ok(value
                .map(|v| {
                    Value::Number(
                        serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0)),
                    )
                })
                .unwrap_or(Value::Null))
        }
        "BLOB" => {
            let value: Option<Vec<u8>> = row.try_get(column_index)?;
            Ok(value
                .map(|v| Value::String(base64::engine::general_purpose::STANDARD.encode(v)))
                .unwrap_or(Value::Null))
        }
        _ => {
            let value: Option<String> = row.try_get(column_index)?;
            Ok(value.map(Value::String).unwrap_or(Value::Null))
        }
    }
}

/// 分页参数
#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Pagination {
    pub fn new(limit: Option<i64>, offset: Option<i64>) -> Self {
        Self { limit, offset }
    }

    pub fn page(page: i64, size: i64) -> Self {
        Self {
            limit: Some(size),
            offset: Some((page - 1) * size),
        }
    }
}

/// 排序参数
#[derive(Debug, Clone)]
pub struct Ordering {
    pub field: String,
    pub desc: bool,
}

impl Ordering {
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            desc: false,
        }
    }

    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            desc: true,
        }
    }
}
