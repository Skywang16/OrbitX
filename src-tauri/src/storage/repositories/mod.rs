/*!
 * 数据访问模块 - 每个表一个简单的结构体，直接使用 sqlx
 *
 * 设计原则：
 * - 无抽象层：直接使用 sqlx，没有 Repository trait
 * - 单一职责：每个结构体对应一张表
 * - 借用优先：使用 &DatabaseManager 而非 Arc
 */

pub mod ai_features;
pub mod ai_models;
pub mod app_preferences;
pub mod audit_logs;
pub mod completion_model;

// ==================== Repository 结构体 ====================
pub use ai_features::AIFeatures;
pub use ai_models::{AIModelConfig, AIModels, AIProvider, ModelType};
pub use app_preferences::AppPreferences;
pub use audit_logs::AuditLogs;
pub use completion_model::CompletionModelRepo;

// ==================== 通用查询参数 ====================

/// 分页参数
#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

impl Pagination {
    pub fn new(limit: i64, offset: i64) -> Self {
        Self { limit, offset }
    }

    pub fn page(page: i64, size: i64) -> Self {
        Self {
            limit: size,
            offset: (page - 1) * size,
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
    pub fn asc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            desc: false,
        }
    }

    pub fn desc(field: &str) -> Self {
        Self {
            field: field.to_string(),
            desc: true,
        }
    }
}
