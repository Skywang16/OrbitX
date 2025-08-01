/*!
 * SQLite数据管理器模块
 *
 * 管理长期数据存储、AI配置和智能查询
 * 实现数据库初始化、迁移系统、加密存储和智能查询功能
 */

use crate::ai::types::AIModelConfig;
use crate::storage::paths::StoragePaths;
use crate::storage::types::{DataQuery, SaveOptions};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng as ChaChaOsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use chrono::{DateTime, Utc};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{
    sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteRow,
        SqliteSynchronous,
    },
    Column, ConnectOptions, Executor, Row, TypeInfo,
};
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// SQLite管理器选项
#[derive(Debug, Clone)]
pub struct SqliteOptions {
    /// 是否启用加密
    pub encryption: bool,
    /// 连接池大小
    pub pool_size: u32,
    /// 连接超时时间（秒）
    pub connection_timeout: u64,
    /// 查询超时时间（秒）
    pub query_timeout: u64,
    /// 是否启用WAL模式
    pub wal_mode: bool,
    /// 缓存大小
    pub cache_size: usize,
}

impl Default for SqliteOptions {
    fn default() -> Self {
        Self {
            encryption: true,
            pool_size: 10,
            connection_timeout: 30,
            query_timeout: 30,
            wal_mode: true,
            cache_size: 1000,
        }
    }
}

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
    pub tags: Option<String>, // JSON数组
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

/// 历史查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryQuery {
    pub command_pattern: Option<String>,
    pub working_directory: Option<String>,
    pub session_id: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 命令历史搜索结果
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

/// AI聊天历史条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIChatHistoryEntry {
    pub id: Option<i64>,
    pub session_id: String,
    pub model_id: String,
    pub role: String, // 'user', 'assistant', 'system'
    pub content: String,
    pub token_count: Option<i32>,
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// AI聊天会话查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistoryQuery {
    pub session_id: Option<String>,
    pub model_id: Option<String>,
    pub role: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Option<i64>,
    pub operation: String,
    pub table_name: String,
    pub record_id: Option<String>,
    pub user_context: Option<String>,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// 加密管理器
pub struct EncryptionManager {
    argon2: Argon2<'static>,
    master_key: Option<Key>,
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
            master_key: None,
        }
    }

    /// 设置主密钥（从用户密码派生）
    pub fn set_master_password(&mut self, password: &str) -> AppResult<()> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!(format!("密钥派生失败: {}", e)))?;

        // 使用密码哈希的前32字节作为加密密钥
        let hash = password_hash.hash.unwrap();
        let key_bytes = hash.as_bytes();
        if key_bytes.len() < 32 {
            return Err(anyhow!("密钥长度不足".to_string()));
        }

        self.master_key = Some(*Key::from_slice(&key_bytes[..32]));
        Ok(())
    }

    /// 加密敏感数据
    pub fn encrypt_data(&self, data: &str) -> AppResult<Vec<u8>> {
        let key = self
            .master_key
            .as_ref()
            .ok_or_else(|| anyhow!("未设置主密钥".to_string()))?;

        let cipher = ChaCha20Poly1305::new(key);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut ChaChaOsRng);

        let ciphertext = cipher
            .encrypt(&nonce, data.as_bytes())
            .map_err(|e| anyhow!(format!("加密失败: {}", e)))?;

        // 组合 nonce + ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// 解密敏感数据
    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> AppResult<String> {
        let key = self
            .master_key
            .as_ref()
            .ok_or_else(|| anyhow!("未设置主密钥".to_string()))?;

        if encrypted_data.len() < 12 {
            return Err(anyhow!("加密数据格式错误".to_string(),));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new(key);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!(format!("解密失败: {}", e)))?;

        String::from_utf8(plaintext).map_err(|e| anyhow!(format!("解密数据格式错误: {}", e)))
    }
}

/// 数据库迁移
#[derive(Clone)]
pub struct Migration {
    pub version: u32,
    pub description: String,
    pub up_sql: String,
    pub down_sql: String,
}

impl Migration {
    pub fn new(
        version: u32,
        description: impl Into<String>,
        up_sql: impl Into<String>,
        down_sql: impl Into<String>,
    ) -> Self {
        Self {
            version,
            description: description.into(),
            up_sql: up_sql.into(),
            down_sql: down_sql.into(),
        }
    }
}

/// SQLite数据管理器
pub struct SqliteManager {
    db_pool: SqlitePool,
    paths: StoragePaths,
    #[allow(dead_code)]
    options: SqliteOptions,
    encryption_manager: Arc<RwLock<EncryptionManager>>,
    cache: Arc<RwLock<LruCache<String, Value>>>,
    migrations: Vec<Migration>,
}

impl SqliteManager {
    /// 创建新的SQLite管理器
    pub async fn new(paths: StoragePaths, options: SqliteOptions) -> AppResult<Self> {
        let db_path = paths.data_dir.join(crate::storage::DATABASE_FILE_NAME);

        // 确保数据目录存在
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建数据目录失败: {}", parent.display()))?;
        }

        // 配置SQLite连接选项
        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(if options.wal_mode {
                SqliteJournalMode::Wal
            } else {
                SqliteJournalMode::Delete
            })
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(options.connection_timeout))
            .disable_statement_logging();

        // 创建连接池
        let db_pool = SqlitePoolOptions::new()
            .max_connections(options.pool_size)
            .acquire_timeout(Duration::from_secs(options.connection_timeout))
            .connect_with(connect_options)
            .await
            .with_context(|| format!("数据库连接失败: {}", db_path.display()))?;

        let cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(options.cache_size).unwrap(),
        )));

        let mut manager = Self {
            db_pool,
            paths,
            options,
            encryption_manager: Arc::new(RwLock::new(EncryptionManager::new())),
            cache,
            migrations: Vec::new(),
        };

        // 初始化迁移
        manager.init_migrations();

        Ok(manager)
    }

    /// 初始化迁移列表
    fn init_migrations(&mut self) {
        self.migrations = vec![Migration::new(
            1,
            "创建基础表结构",
            include_str!("../../migrations/001_initial_schema.sql"),
            "DROP TABLE IF EXISTS ai_model_usage_stats;
                 DROP TABLE IF EXISTS ai_features;
                 DROP TABLE IF EXISTS terminal_sessions;
                 DROP TABLE IF EXISTS command_search;
                 DROP TABLE IF EXISTS ai_chat_history;
                 DROP TABLE IF EXISTS command_usage_stats;
                 DROP TABLE IF EXISTS command_history;
                 DROP TABLE IF EXISTS ai_models;
                 DROP TABLE IF EXISTS schema_migrations;",
        )];
    }

    /// 初始化数据库
    pub async fn initialize_database(&self) -> AppResult<()> {
        info!("初始化SQLite数据库");

        // 设置默认主密钥（用于加密敏感数据）
        self.set_default_master_key().await?;

        // 创建迁移表
        self.create_migration_table().await?;

        // 执行迁移
        self.run_migrations().await?;

        // 插入默认数据
        self.insert_default_data().await?;

        info!("数据库初始化完成");
        Ok(())
    }

    /// 设置默认主密钥
    async fn set_default_master_key(&self) -> AppResult<()> {
        let mut encryption_manager = self.encryption_manager.write().await;

        // 检查是否已经设置了主密钥
        if encryption_manager.master_key.is_some() {
            debug!("主密钥已设置，跳过默认密钥设置");
            return Ok(());
        }

        // 使用应用程序特定的默认密钥
        // 在生产环境中，这应该从安全的配置文件或环境变量中读取
        let default_password = "termx-default-encryption-key-2024";
        encryption_manager.set_master_password(default_password)?;

        info!("默认主密钥设置完成");
        Ok(())
    }

    /// 创建迁移表
    async fn create_migration_table(&self) -> AppResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
        "#;

        self.db_pool.execute(sql).await.with_context(|| {
            format!(
                "创建迁移表失败: {}",
                self.paths
                    .data_dir
                    .join(crate::storage::DATABASE_FILE_NAME)
                    .display()
            )
        })?;

        Ok(())
    }

    /// 运行数据库迁移
    async fn run_migrations(&self) -> AppResult<()> {
        let current_version = self.get_current_schema_version().await?;
        info!("当前数据库版本: {}", current_version);

        for migration in &self.migrations {
            if migration.version > current_version {
                info!("执行迁移 v{}: {}", migration.version, migration.description);

                // 开始事务
                let mut tx = self
                    .db_pool
                    .begin()
                    .await
                    .map_err(|e| anyhow!("开始迁移事务失败: {}", e))?;

                // 执行迁移SQL
                tx.execute(migration.up_sql.as_str())
                    .await
                    .map_err(|e| anyhow!("执行迁移SQL失败: {}", e))?;

                // 记录迁移
                let record_sql =
                    "INSERT INTO schema_migrations (version, description) VALUES (?, ?)";
                tx.execute(
                    sqlx::query(record_sql)
                        .bind(migration.version as i64)
                        .bind(&migration.description),
                )
                .await
                .map_err(|e| anyhow!("记录迁移失败: {}", e))?;

                // 提交事务
                tx.commit()
                    .await
                    .map_err(|e| anyhow!("提交迁移事务失败: {}", e))?;

                info!("迁移 v{} 完成", migration.version);
            }
        }

        Ok(())
    }

    /// 获取当前数据库版本
    async fn get_current_schema_version(&self) -> AppResult<u32> {
        let sql = "SELECT MAX(version) as version FROM schema_migrations";

        let row = self
            .db_pool
            .fetch_optional(sql)
            .await
            .map_err(|e| anyhow!("查询数据库版本失败: {}", e))?;

        match row {
            Some(row) => {
                let version: Option<i64> = row
                    .try_get("version")
                    .map_err(|e| anyhow!("解析版本号失败: {}", e))?;
                Ok(version.unwrap_or(0) as u32)
            }
            None => Ok(0),
        }
    }

    /// 插入默认数据
    async fn insert_default_data(&self) -> AppResult<()> {
        // 插入默认AI功能配置
        let features = vec![
            ("chat", true, r#"{"max_history": 100, "auto_save": true}"#),
            ("explanation", true, r#"{"auto_explain": false}"#),
            ("command_search", true, r#"{"max_results": 50}"#),
        ];

        for (feature_name, enabled, config_json) in features {
            let sql = r#"
                INSERT OR IGNORE INTO ai_features (feature_name, enabled, config_json)
                VALUES (?, ?, ?)
            "#;

            self.db_pool
                .execute(
                    sqlx::query(sql)
                        .bind(feature_name)
                        .bind(enabled)
                        .bind(config_json),
                )
                .await
                .map_err(|e| anyhow!("插入默认AI功能配置失败: {}", e))?;
        }

        Ok(())
    }

    /// 迁移到指定版本
    pub async fn migrate_to_version(&self, target_version: u32) -> AppResult<()> {
        let current_version = self.get_current_schema_version().await?;

        if target_version == current_version {
            return Ok(());
        }

        if target_version > current_version {
            // 向上迁移
            for migration in &self.migrations {
                if migration.version > current_version && migration.version <= target_version {
                    info!(
                        "执行向上迁移 v{}: {}",
                        migration.version, migration.description
                    );
                    self.apply_migration(migration, true).await?;
                }
            }
        } else {
            // 向下迁移
            let mut migrations = self.migrations.clone();
            migrations.sort_by(|a, b| b.version.cmp(&a.version)); // 降序排列

            for migration in &migrations {
                if migration.version <= current_version && migration.version > target_version {
                    info!(
                        "执行向下迁移 v{}: {}",
                        migration.version, migration.description
                    );
                    self.apply_migration(migration, false).await?;
                }
            }
        }

        Ok(())
    }

    /// 应用单个迁移
    async fn apply_migration(&self, migration: &Migration, is_up: bool) -> AppResult<()> {
        let mut tx = self
            .db_pool
            .begin()
            .await
            .map_err(|e| anyhow!("开始迁移事务失败: {}", e))?;

        let sql = if is_up {
            &migration.up_sql
        } else {
            &migration.down_sql
        };

        tx.execute(sql.as_str())
            .await
            .map_err(|e| anyhow!("执行迁移SQL失败: {}", e))?;

        if is_up {
            // 记录迁移
            let record_sql = "INSERT INTO schema_migrations (version, description) VALUES (?, ?)";
            tx.execute(
                sqlx::query(record_sql)
                    .bind(migration.version as i64)
                    .bind(&migration.description),
            )
            .await
            .map_err(|e| anyhow!("记录迁移失败: {}", e))?;
        } else {
            // 删除迁移记录
            let delete_sql = "DELETE FROM schema_migrations WHERE version = ?";
            tx.execute(sqlx::query(delete_sql).bind(migration.version as i64))
                .await
                .map_err(|e| anyhow!("删除迁移记录失败: {}", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| anyhow!("提交迁移事务失败: {}", e))?;

        Ok(())
    }

    /// 查询数据
    pub async fn query_data(&self, query: &DataQuery) -> AppResult<Vec<Value>> {
        debug!("执行数据查询: {}", query.query);

        // 检查缓存
        let cache_key = format!("query:{}", serde_json::to_string(query).unwrap_or_default());
        {
            let cache = self.cache.read().await;
            if let Some(cached_result) = cache.peek(&cache_key) {
                debug!("从缓存返回查询结果");
                if let Ok(results) = serde_json::from_value::<Vec<Value>>(cached_result.clone()) {
                    return Ok(results);
                }
            }
        }

        // 构建SQL查询
        let mut sql = query.query.clone();

        // 添加排序
        if let Some(order_by) = &query.order_by {
            sql.push_str(&format!(
                " ORDER BY {} {}",
                order_by,
                if query.desc { "DESC" } else { "ASC" }
            ));
        }

        // 添加限制和偏移
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        // 执行查询
        let rows = self
            .db_pool
            .fetch_all(sql.as_str())
            .await
            .map_err(|e| anyhow!("查询执行失败: {}", e))?;

        // 转换结果
        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();

            // 获取所有列
            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                let value = self.extract_value_from_row(&row, i)?;
                obj.insert(column_name.to_string(), value);
            }

            results.push(Value::Object(obj));
        }

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.put(
                cache_key,
                serde_json::to_value(&results).unwrap_or(Value::Null),
            );
        }

        Ok(results)
    }

    /// 从行中提取值
    fn extract_value_from_row(&self, row: &SqliteRow, column_index: usize) -> AppResult<Value> {
        let column = &row.columns()[column_index];
        let column_type = column.type_info();

        // 根据列类型提取值
        match column_type.name() {
            "TEXT" => {
                let value: Option<String> = row
                    .try_get(column_index)
                    .map_err(|e| anyhow!("提取TEXT值失败: {}", e))?;
                Ok(value.map(Value::String).unwrap_or(Value::Null))
            }
            "INTEGER" => {
                let value: Option<i64> = row
                    .try_get(column_index)
                    .map_err(|e| anyhow!("提取INTEGER值失败: {}", e))?;
                Ok(value
                    .map(|v| Value::Number(v.into()))
                    .unwrap_or(Value::Null))
            }
            "REAL" => {
                let value: Option<f64> = row
                    .try_get(column_index)
                    .map_err(|e| anyhow!("提取REAL值失败: {}", e))?;
                Ok(value
                    .map(|v| {
                        Value::Number(
                            serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0)),
                        )
                    })
                    .unwrap_or(Value::Null))
            }
            "BLOB" => {
                let value: Option<Vec<u8>> = row
                    .try_get(column_index)
                    .map_err(|e| anyhow!("提取BLOB值失败: {}", e))?;
                Ok(value
                    .map(|v| Value::String(general_purpose::STANDARD.encode(v)))
                    .unwrap_or(Value::Null))
            }
            _ => {
                // 尝试作为字符串提取
                let value: Option<String> = row
                    .try_get(column_index)
                    .map_err(|e| anyhow!("提取未知类型值失败: {}", e))?;
                Ok(value.map(Value::String).unwrap_or(Value::Null))
            }
        }
    }

    /// 保存数据
    pub async fn save_data(&self, data: &Value, options: &SaveOptions) -> AppResult<()> {
        let table = options
            .table
            .as_ref()
            .ok_or_else(|| anyhow!("未指定目标表".to_string()))?;

        debug!("保存数据到表: {}", table);

        // 清除相关缓存
        self.clear_table_cache(table).await;

        // 根据表名选择保存策略
        match table.as_str() {
            "ai_models" => self.save_ai_model_data(data, options).await,
            "command_history" => self.save_command_history_data(data, options).await,
            "ai_chat_history" => self.save_ai_chat_history_data(data, options).await,
            _ => self.save_generic_data(table, data, options).await,
        }
    }

    /// 保存AI模型数据
    async fn save_ai_model_data(&self, data: &Value, options: &SaveOptions) -> AppResult<()> {
        debug!("开始保存AI模型数据: {:?}", data);

        let model: AIModelConfig = serde_json::from_value(data.clone())
            .map_err(|e| anyhow!(format!("AI模型数据格式错误: {}", e)))?;

        debug!(
            "解析AI模型配置成功: id={}, name={}, provider={:?}",
            model.id, model.name, model.provider
        );

        let operation = if options.overwrite {
            "UPDATE"
        } else {
            "CREATE"
        };

        // 加密API密钥
        let encrypted_api_key = if !model.api_key.is_empty() {
            debug!("加密API密钥");
            let encryption_manager = self.encryption_manager.read().await;
            Some(encryption_manager.encrypt_data(&model.api_key)?)
        } else {
            debug!("API密钥为空，跳过加密");
            None
        };

        // 序列化options为JSON字符串
        let config_json = model
            .options
            .as_ref()
            .map(|opts| serde_json::to_string(opts).unwrap_or_default());

        debug!("配置JSON: {:?}", config_json);

        let now = Utc::now();
        let sql = if options.overwrite {
            r#"
                INSERT OR REPLACE INTO ai_models
                (id, name, provider, api_url, api_key_encrypted, model_name, is_default, enabled, config_json, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        } else {
            r#"
                INSERT INTO ai_models
                (id, name, provider, api_url, api_key_encrypted, model_name, is_default, enabled, config_json, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        };

        let provider_str = format!("{:?}", model.provider);
        debug!("准备执行SQL: {}", sql);
        debug!(
            "绑定参数: id={}, name={}, provider={}, api_url={}, model={}, is_default={:?}",
            model.id, model.name, provider_str, model.api_url, model.model, model.is_default
        );

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&model.id)
                    .bind(&model.name)
                    .bind(provider_str) // 序列化枚举
                    .bind(&model.api_url)
                    .bind(encrypted_api_key)
                    .bind(&model.model) // 这个字段对应数据库中的model_name
                    .bind(model.is_default.unwrap_or(false))
                    .bind(true) // enabled - ai模块中没有这个字段，默认为true
                    .bind(config_json)
                    .bind(now)
                    .bind(now),
            )
            .await;

        debug!("SQL执行结果: {:?}", result);

        match result {
            Ok(_) => {
                // 记录成功的审计日志
                let details = format!("AI模型 '{}' ({:?}) 保存成功", model.name, model.provider);
                self.log_audit_event(
                    operation,
                    "ai_models",
                    Some(&model.id),
                    None,
                    &details,
                    true,
                    None,
                )
                .await?;
                Ok(())
            }
            Err(e) => {
                // 记录失败的审计日志
                let error_msg = format!("保存AI模型失败: {}", e);
                let details = format!("尝试保存AI模型 '{}' ({:?})", model.name, model.provider);
                self.log_audit_event(
                    operation,
                    "ai_models",
                    Some(&model.id),
                    None,
                    &details,
                    false,
                    Some(&error_msg),
                )
                .await?;
                Err(anyhow!("{}", error_msg))
            }
        }
    }

    /// 保存命令历史数据
    async fn save_command_history_data(
        &self,
        data: &Value,
        _options: &SaveOptions,
    ) -> AppResult<()> {
        let entry: CommandHistoryEntry = serde_json::from_value(data.clone())
            .map_err(|e| anyhow!(format!("命令历史数据格式错误: {}", e)))?;

        let sql = r#"
            INSERT INTO command_history 
            (command, working_directory, exit_code, output, duration_ms, executed_at, session_id, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        self.db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&entry.command)
                    .bind(&entry.working_directory)
                    .bind(entry.exit_code)
                    .bind(&entry.output)
                    .bind(entry.duration_ms)
                    .bind(entry.executed_at)
                    .bind(&entry.session_id)
                    .bind(&entry.tags),
            )
            .await
            .map_err(|e| anyhow!("保存命令历史失败: {}", e))?;

        // 更新使用统计
        self.update_command_usage_stats(&entry).await?;

        Ok(())
    }

    /// 保存AI聊天历史数据
    async fn save_ai_chat_history_data(
        &self,
        data: &Value,
        _options: &SaveOptions,
    ) -> AppResult<()> {
        debug!("开始保存AI聊天历史数据: {:?}", data);

        let entry: AIChatHistoryEntry = serde_json::from_value(data.clone())
            .map_err(|e| anyhow!(format!("AI聊天历史数据格式错误: {}", e)))?;

        debug!(
            "解析AI聊天历史成功: session_id={}, model_id={}, role={}",
            entry.session_id, entry.model_id, entry.role
        );

        let sql = r#"
            INSERT INTO ai_chat_history
            (session_id, model_id, role, content, token_count, metadata_json, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&entry.session_id)
                    .bind(&entry.model_id)
                    .bind(&entry.role)
                    .bind(&entry.content)
                    .bind(entry.token_count)
                    .bind(&entry.metadata_json)
                    .bind(entry.created_at),
            )
            .await;

        match result {
            Ok(_) => {
                debug!("AI聊天历史保存成功");
                Ok(())
            }
            Err(e) => {
                error!("保存AI聊天历史失败: {}", e);
                Err(anyhow!("保存AI聊天历史失败: {}", e))
            }
        }
    }

    /// 保存通用数据
    async fn save_generic_data(
        &self,
        table: &str,
        _data: &Value,
        _options: &SaveOptions,
    ) -> AppResult<()> {
        // 这是一个简化的通用保存实现
        // 实际应用中可能需要更复杂的逻辑来处理不同的数据结构
        warn!("使用通用数据保存方法保存到表: {}", table);

        // 这里可以实现通用的JSON数据保存逻辑
        // 或者返回错误要求使用特定的保存方法
        Err(anyhow!(format!(
            "表 {} 不支持通用数据保存，请使用特定的保存方法",
            table
        )))
    }

    /// 更新命令使用统计
    async fn update_command_usage_stats(&self, entry: &CommandHistoryEntry) -> AppResult<()> {
        let command_hash = format!("{:x}", md5::compute(&entry.command));

        let sql = r#"
            INSERT INTO command_usage_stats (command_hash, command, working_directory, usage_count, last_used, avg_duration_ms)
            VALUES (?, ?, ?, 1, ?, ?)
            ON CONFLICT(command_hash, working_directory) DO UPDATE SET
                usage_count = usage_count + 1,
                last_used = excluded.last_used,
                avg_duration_ms = (avg_duration_ms * (usage_count - 1) + excluded.avg_duration_ms) / usage_count
        "#;

        self.db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&command_hash)
                    .bind(&entry.command)
                    .bind(&entry.working_directory)
                    .bind(entry.executed_at)
                    .bind(entry.duration_ms.unwrap_or(0)),
            )
            .await
            .map_err(|e| anyhow!("更新命令使用统计失败: {}", e))?;

        Ok(())
    }

    /// 清除表相关的缓存
    async fn clear_table_cache(&self, table: &str) {
        let mut cache = self.cache.write().await;
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.contains(table))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
        }
    }

    /// 获取数据库连接池的引用
    pub fn pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    /// 获取加密管理器的引用
    pub fn encryption_manager(&self) -> Arc<RwLock<EncryptionManager>> {
        self.encryption_manager.clone()
    }

    /// 清除所有缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), cache.cap().get())
    }

    // ========================================================================
    // AI配置管理方法
    // ========================================================================

    /// 获取所有AI模型配置
    pub async fn get_ai_models(&self) -> AppResult<Vec<AIModelConfig>> {
        let cache_key = "ai_models_all".to_string();

        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached_value) = cache.peek(&cache_key) {
                if let Ok(models) =
                    serde_json::from_value::<Vec<AIModelConfig>>(cached_value.clone())
                {
                    debug!("从缓存获取AI模型配置");
                    return Ok(models);
                }
            }
        }

        let sql = r#"
            SELECT id, name, provider, api_url, api_key_encrypted, model_name,
                   is_default, enabled, config_json, created_at, updated_at
            FROM ai_models
            ORDER BY is_default DESC, name ASC
        "#;

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(sql))
            .await
            .map_err(|e| anyhow!("查询AI模型失败: {}", e))?;

        let encryption_manager = self.encryption_manager.read().await;
        let mut models = Vec::new();

        for row in rows {
            let model = self.row_to_ai_model_config(&row, &encryption_manager)?;
            models.push(model);
        }

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.put(cache_key, serde_json::to_value(&models).unwrap_or_default());
        }

        Ok(models)
    }

    /// 获取默认AI模型
    pub async fn get_default_ai_model(&self) -> AppResult<Option<AIModelConfig>> {
        let models = self.get_ai_models().await?;
        Ok(models.into_iter().find(|m| m.is_default.unwrap_or(false)))
    }

    /// 设置默认AI模型
    pub async fn set_default_ai_model(&self, model_id: &str) -> AppResult<()> {
        let mut tx = self
            .db_pool
            .begin()
            .await
            .map_err(|e| anyhow!("开始事务失败: {}", e))?;

        // 清除所有默认标记
        let clear_sql = "UPDATE ai_models SET is_default = FALSE";
        tx.execute(sqlx::query(clear_sql))
            .await
            .map_err(|e| anyhow!("清除默认模型失败: {}", e))?;

        // 设置新的默认模型
        let set_sql = "UPDATE ai_models SET is_default = TRUE WHERE id = ?";
        let result = tx
            .execute(sqlx::query(set_sql).bind(model_id))
            .await
            .map_err(|e| anyhow!("设置默认模型失败: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!(format!("模型ID不存在: {}", model_id)));
        }

        tx.commit()
            .await
            .map_err(|e| anyhow!("提交事务失败: {}", e))?;

        // 清除相关缓存
        self.clear_table_cache("ai_models").await;

        Ok(())
    }

    /// 保存AI聊天消息
    pub async fn save_chat_message(&self, entry: &AIChatHistoryEntry) -> AppResult<i64> {
        debug!(
            "保存AI聊天消息: session_id={}, role={}",
            entry.session_id, entry.role
        );

        let sql = r#"
            INSERT INTO ai_chat_history
            (session_id, model_id, role, content, token_count, metadata_json, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&entry.session_id)
                    .bind(&entry.model_id)
                    .bind(&entry.role)
                    .bind(&entry.content)
                    .bind(entry.token_count)
                    .bind(&entry.metadata_json)
                    .bind(entry.created_at),
            )
            .await
            .map_err(|e| anyhow!("保存AI聊天消息失败: {}", e))?;

        // 清除相关缓存
        self.clear_table_cache("ai_chat_history").await;

        Ok(result.last_insert_rowid())
    }

    /// 查询AI聊天历史
    pub async fn get_chat_history(
        &self,
        query: &ChatHistoryQuery,
    ) -> AppResult<Vec<AIChatHistoryEntry>> {
        debug!("查询AI聊天历史: session_id={:?}", query.session_id);

        // 构建基础查询
        let mut sql = String::from(
            r#"
            SELECT id, session_id, model_id, role, content, token_count, metadata_json, created_at
            FROM ai_chat_history
            WHERE 1=1
        "#,
        );

        // 添加查询条件
        if let Some(session_id) = &query.session_id {
            sql.push_str(&format!(
                " AND session_id = '{}'",
                session_id.replace("'", "''")
            ));
        }

        if let Some(model_id) = &query.model_id {
            sql.push_str(&format!(
                " AND model_id = '{}'",
                model_id.replace("'", "''")
            ));
        }

        if let Some(role) = &query.role {
            sql.push_str(&format!(" AND role = '{}'", role.replace("'", "''")));
        }

        if let Some(date_from) = &query.date_from {
            sql.push_str(&format!(
                " AND created_at >= '{}'",
                date_from.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if let Some(date_to) = &query.date_to {
            sql.push_str(&format!(
                " AND created_at <= '{}'",
                date_to.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        sql.push_str(" ORDER BY created_at ASC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        // 执行查询
        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .map_err(|e| anyhow!("查询AI聊天历史失败: {}", e))?;

        let entries: Vec<AIChatHistoryEntry> = rows
            .iter()
            .map(|row| self.row_to_chat_history_entry(row))
            .collect();

        debug!("查询到 {} 条AI聊天历史记录", entries.len());
        Ok(entries)
    }

    /// 获取所有会话列表
    pub async fn get_chat_sessions(&self) -> AppResult<Vec<String>> {
        let sql = r#"
            SELECT DISTINCT session_id, MAX(created_at) as last_activity
            FROM ai_chat_history
            GROUP BY session_id
            ORDER BY last_activity DESC
        "#;

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(sql))
            .await
            .map_err(|e| anyhow!("获取会话列表失败: {}", e))?;

        let sessions: Vec<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("session_id"))
            .collect();

        Ok(sessions)
    }

    /// 清除AI聊天历史
    pub async fn clear_chat_history(&self, session_id: Option<&str>) -> AppResult<u64> {
        debug!("清除AI聊天历史: session_id={:?}", session_id);

        let (_sql, affected_rows) = if let Some(session_id) = session_id {
            let _sql = "DELETE FROM ai_chat_history WHERE session_id = ?";
            let result = self
                .db_pool
                .execute(sqlx::query(_sql).bind(session_id))
                .await
                .map_err(|e| anyhow!("清除指定会话聊天历史失败: {}", e))?;
            (_sql, result.rows_affected())
        } else {
            let _sql = "DELETE FROM ai_chat_history";
            let result = self
                .db_pool
                .execute(sqlx::query(_sql))
                .await
                .map_err(|e| anyhow!("清除所有聊天历史失败: {}", e))?;
            (_sql, result.rows_affected())
        };

        // 清除相关缓存
        self.clear_table_cache("ai_chat_history").await;

        debug!("清除了 {} 条AI聊天历史记录", affected_rows);
        Ok(affected_rows)
    }

    /// 删除AI模型
    pub async fn delete_ai_model(&self, model_id: &str) -> AppResult<()> {
        // 先获取模型信息用于审计日志
        let model_info = self
            .db_pool
            .fetch_optional(
                sqlx::query("SELECT name, provider FROM ai_models WHERE id = ?").bind(model_id),
            )
            .await
            .map_err(|e| anyhow!("查询AI模型信息失败: {}", e))?;

        let sql = "DELETE FROM ai_models WHERE id = ?";
        let result = self.db_pool.execute(sqlx::query(sql).bind(model_id)).await;

        match result {
            Ok(query_result) => {
                if query_result.rows_affected() == 0 {
                    let error_msg = format!("模型ID不存在: {}", model_id);
                    self.log_audit_event(
                        "DELETE",
                        "ai_models",
                        Some(model_id),
                        None,
                        &error_msg,
                        false,
                        Some(&error_msg),
                    )
                    .await?;
                    return Err(anyhow!(error_msg));
                }

                // 记录成功的审计日志
                let details = if let Some(row) = model_info {
                    let name: String = row.get("name");
                    let provider: String = row.get("provider");
                    format!("AI模型 '{}' ({}) 删除成功", name, provider)
                } else {
                    format!("AI模型 {} 删除成功", model_id)
                };

                self.log_audit_event(
                    "DELETE",
                    "ai_models",
                    Some(model_id),
                    None,
                    &details,
                    true,
                    None,
                )
                .await?;

                // 清除相关缓存
                self.clear_table_cache("ai_models").await;

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("删除AI模型失败: {}", e);
                let details = format!("尝试删除AI模型 {}", model_id);
                self.log_audit_event(
                    "DELETE",
                    "ai_models",
                    Some(model_id),
                    None,
                    &details,
                    false,
                    Some(&error_msg),
                )
                .await?;
                Err(anyhow!("{}", error_msg))
            }
        }
    }

    /// 设置主密钥（用于加密敏感数据）
    pub async fn set_master_password(&self, password: &str) -> AppResult<()> {
        let mut encryption_manager = self.encryption_manager.write().await;
        encryption_manager.set_master_password(password)?;
        info!("主密钥设置成功");

        // 记录审计日志
        self.log_audit_event(
            "SET_MASTER_KEY",
            "encryption",
            None,
            None,
            "主密钥设置",
            true,
            None,
        )
        .await?;

        Ok(())
    }

    /// 将数据库行转换为AI模型配置
    fn row_to_ai_model_config(
        &self,
        row: &SqliteRow,
        encryption_manager: &EncryptionManager,
    ) -> AppResult<AIModelConfig> {
        let api_key =
            if let Some(encrypted_data) = row.get::<Option<Vec<u8>>, _>("api_key_encrypted") {
                encryption_manager
                    .decrypt_data(&encrypted_data)
                    .unwrap_or_default()
            } else {
                String::new()
            };

        let provider_str: String = row.get("provider");
        let provider = match provider_str.as_str() {
            "OpenAI" => crate::ai::types::AIProvider::OpenAI,
            "Claude" => crate::ai::types::AIProvider::Claude,
            "Local" => crate::ai::types::AIProvider::Custom, // 将旧的 Local 映射到 Custom
            _ => crate::ai::types::AIProvider::Custom,
        };

        let options = if let Some(config_json) = row.get::<Option<String>, _>("config_json") {
            serde_json::from_str(&config_json).ok()
        } else {
            None
        };

        Ok(AIModelConfig {
            id: row.get("id"),
            name: row.get("name"),
            provider,
            api_url: row.get("api_url"),
            api_key,
            model: row.get("model_name"),
            is_default: Some(row.get("is_default")),
            options,
        })
    }

    /// 将数据库行转换为命令历史条目
    fn row_to_command_history_entry(&self, row: &SqliteRow) -> CommandHistoryEntry {
        CommandHistoryEntry {
            id: row.get("id"),
            command: row.get("command"),
            working_directory: row.get("working_directory"),
            exit_code: row.get("exit_code"),
            output: row.get("output"),
            duration_ms: row.get("duration_ms"),
            executed_at: row.get("executed_at"),
            session_id: row.get("session_id"),
            tags: row.get("tags"),
        }
    }

    /// 将数据库行转换为AI聊天历史条目
    fn row_to_chat_history_entry(&self, row: &SqliteRow) -> AIChatHistoryEntry {
        AIChatHistoryEntry {
            id: row.get("id"),
            session_id: row.get("session_id"),
            model_id: row.get("model_id"),
            role: row.get("role"),
            content: row.get("content"),
            token_count: row.get("token_count"),
            metadata_json: row.get("metadata_json"),
            created_at: row.get("created_at"),
        }
    }

    // ========================================================================
    // 审计日志方法
    // ========================================================================

    /// 记录审计日志
    pub async fn log_audit_event(
        &self,
        operation: &str,
        table_name: &str,
        record_id: Option<&str>,
        user_context: Option<&str>,
        details: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        let sql = r#"
            INSERT INTO audit_logs (operation, table_name, record_id, user_context, details, success, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        self.db_pool
            .execute(
                sqlx::query(sql)
                    .bind(operation)
                    .bind(table_name)
                    .bind(record_id)
                    .bind(user_context)
                    .bind(details)
                    .bind(success)
                    .bind(error_message),
            )
            .await
            .map_err(|e| anyhow!("记录审计日志失败: {}", e))?;

        Ok(())
    }

    /// 查询审计日志
    pub async fn get_audit_logs(
        &self,
        table_name: Option<&str>,
        operation: Option<&str>,
        limit: Option<i64>,
    ) -> AppResult<Vec<AuditLogEntry>> {
        let mut sql = String::from(
            r#"
            SELECT id, operation, table_name, record_id, user_context, details,
                   timestamp, success, error_message
            FROM audit_logs
            WHERE 1=1
        "#,
        );

        if let Some(table) = table_name {
            sql.push_str(&format!(" AND table_name = '{}'", table));
        }

        if let Some(op) = operation {
            sql.push_str(&format!(" AND operation = '{}'", op));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        if let Some(limit_val) = limit {
            sql.push_str(&format!(" LIMIT {}", limit_val));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .map_err(|e| anyhow!("查询审计日志失败: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            let entry = AuditLogEntry {
                id: row.get("id"),
                operation: row.get("operation"),
                table_name: row.get("table_name"),
                record_id: row.get("record_id"),
                user_context: row.get("user_context"),
                details: row.get("details"),
                timestamp: row.get("timestamp"),
                success: row.get("success"),
                error_message: row.get("error_message"),
            };
            entries.push(entry);
        }

        Ok(entries)
    }

    // ========================================================================
    // 命令历史和智能查询方法
    // ========================================================================

    /// 查询命令历史
    pub async fn query_command_history(
        &self,
        query: &HistoryQuery,
    ) -> AppResult<Vec<CommandHistoryEntry>> {
        // 构建基础查询
        let mut sql = String::from(
            r#"
            SELECT id, command, working_directory, exit_code, output,
                   duration_ms, executed_at, session_id, tags
            FROM command_history
            WHERE 1=1
        "#,
        );

        // 简化版本：使用字符串拼接构建查询（在生产环境中应该使用参数化查询）
        if let Some(pattern) = &query.command_pattern {
            sql.push_str(&format!(
                " AND command LIKE '%{}%'",
                pattern.replace("'", "''")
            ));
        }

        if let Some(working_dir) = &query.working_directory {
            sql.push_str(&format!(
                " AND working_directory = '{}'",
                working_dir.replace("'", "''")
            ));
        }

        if let Some(session_id) = &query.session_id {
            sql.push_str(&format!(
                " AND session_id = '{}'",
                session_id.replace("'", "''")
            ));
        }

        if let Some(date_from) = &query.date_from {
            sql.push_str(&format!(
                " AND executed_at >= '{}'",
                date_from.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if let Some(date_to) = &query.date_to {
            sql.push_str(&format!(
                " AND executed_at <= '{}'",
                date_to.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        sql.push_str(" ORDER BY executed_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        // 执行查询
        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .map_err(|e| anyhow!("查询命令历史失败: {}", e))?;

        let entries: Vec<CommandHistoryEntry> = rows
            .iter()
            .map(|row| self.row_to_command_history_entry(row))
            .collect();

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

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(sql).bind(search_query))
            .await
            .map_err(|e| anyhow!("全文搜索失败: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            let result = CommandSearchResult {
                id: row.get("id"),
                command: row.get("command"),
                working_directory: row.get("working_directory"),
                output: row.get::<Option<String>, _>("output"),
                executed_at: row.get("executed_at"),
                command_snippet: row.get::<Option<String>, _>("command_snippet"),
                output_snippet: row.get::<Option<String>, _>("output_snippet"),
                relevance_score: 1.0, // FTS5会自动排序，这里设为固定值
            };
            results.push(result);
        }

        Ok(results)
    }

    /// 获取使用统计
    pub async fn get_usage_statistics(&self) -> AppResult<UsageStats> {
        // 总命令数
        let total_commands: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_history")
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| anyhow!("查询总命令数失败: {}", e))?;

        // 唯一命令数
        let unique_commands: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT command) FROM command_history")
                .fetch_one(&self.db_pool)
                .await
                .map_err(|e| anyhow!("查询唯一命令数失败: {}", e))?;

        // 平均执行时间
        let avg_execution_time: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(duration_ms) FROM command_history WHERE duration_ms IS NOT NULL",
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| anyhow!("查询平均执行时间失败: {}", e))?;

        // 最常用命令
        let most_used_rows = self
            .db_pool
            .fetch_all(sqlx::query(
                r#"
                SELECT command, COUNT(*) as usage_count
                FROM command_history
                GROUP BY command
                ORDER BY usage_count DESC
                LIMIT 10
            "#,
            ))
            .await
            .map_err(|e| anyhow!("查询最常用命令失败: {}", e))?;

        let most_used_commands: Vec<(String, i64)> = most_used_rows
            .into_iter()
            .map(|row| (row.get("command"), row.get("usage_count")))
            .collect();

        // 最近活动
        let recent_rows = self
            .db_pool
            .fetch_all(sqlx::query(
                r#"
                SELECT id, command, working_directory, exit_code, output,
                       duration_ms, executed_at, session_id, tags
                FROM command_history
                ORDER BY executed_at DESC
                LIMIT 20
            "#,
            ))
            .await
            .map_err(|e| anyhow!("查询最近活动失败: {}", e))?;

        let recent_activity: Vec<CommandHistoryEntry> = recent_rows
            .into_iter()
            .map(|row| self.row_to_command_history_entry(&row))
            .collect();

        Ok(UsageStats {
            total_commands,
            unique_commands,
            avg_execution_time: avg_execution_time.unwrap_or(0.0),
            most_used_commands,
            recent_activity,
        })
    }

    /// 获取命令使用趋势（按日期分组）
    pub async fn get_command_usage_trends(&self, days: i64) -> AppResult<Vec<(String, i64)>> {
        let sql = r#"
            SELECT DATE(executed_at) as date, COUNT(*) as count
            FROM command_history
            WHERE executed_at >= datetime('now', '-' || ? || ' days')
            GROUP BY DATE(executed_at)
            ORDER BY date DESC
        "#;

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(sql).bind(days))
            .await
            .map_err(|e| anyhow!("查询命令使用趋势失败: {}", e))?;

        let trends: Vec<(String, i64)> = rows
            .into_iter()
            .map(|row| (row.get("date"), row.get("count")))
            .collect();

        Ok(trends)
    }

    /// 获取最常用的工作目录
    pub async fn get_popular_directories(
        &self,
        limit: Option<i64>,
    ) -> AppResult<Vec<(String, i64)>> {
        let mut sql = String::from(
            r#"
            SELECT working_directory, COUNT(*) as usage_count
            FROM command_history
            GROUP BY working_directory
            ORDER BY usage_count DESC
        "#,
        );

        if let Some(limit_val) = limit {
            sql.push_str(&format!(" LIMIT {}", limit_val));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .map_err(|e| anyhow!("查询常用目录失败: {}", e))?;

        let directories: Vec<(String, i64)> = rows
            .into_iter()
            .map(|row| (row.get("working_directory"), row.get("usage_count")))
            .collect();

        Ok(directories)
    }

    /// 获取命令执行时间分析
    pub async fn get_command_performance_analysis(&self) -> AppResult<HashMap<String, f64>> {
        let sql = r#"
            SELECT command, AVG(duration_ms) as avg_duration
            FROM command_history
            WHERE duration_ms IS NOT NULL AND duration_ms > 0
            GROUP BY command
            HAVING COUNT(*) >= 3  -- 至少执行过3次的命令
            ORDER BY avg_duration DESC
            LIMIT 20
        "#;

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(sql))
            .await
            .map_err(|e| anyhow!("查询命令性能分析失败: {}", e))?;

        let mut analysis = HashMap::new();
        for row in rows {
            let command: String = row.get("command");
            let avg_duration: f64 = row.get("avg_duration");
            analysis.insert(command, avg_duration);
        }

        Ok(analysis)
    }

    /// 获取错误命令分析
    pub async fn get_error_command_analysis(
        &self,
        limit: Option<i64>,
    ) -> AppResult<Vec<(String, i64, f64)>> {
        let mut sql = String::from(
            r#"
            SELECT
                command,
                COUNT(*) as total_count,
                COUNT(CASE WHEN exit_code != 0 THEN 1 END) as error_count,
                ROUND(COUNT(CASE WHEN exit_code != 0 THEN 1 END) * 100.0 / COUNT(*), 2) as error_rate
            FROM command_history
            WHERE exit_code IS NOT NULL
            GROUP BY command
            HAVING total_count >= 3  -- 至少执行过3次的命令
            ORDER BY error_rate DESC, error_count DESC
        "#,
        );

        if let Some(limit_val) = limit {
            sql.push_str(&format!(" LIMIT {}", limit_val));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .map_err(|e| anyhow!("查询错误命令分析失败: {}", e))?;

        let mut analysis = Vec::new();
        for row in rows {
            let command: String = row.get("command");
            let error_count: i64 = row.get("error_count");
            let error_rate: f64 = row.get("error_rate");
            analysis.push((command, error_count, error_rate));
        }

        Ok(analysis)
    }

    /// 智能命令推荐（基于当前工作目录和历史使用）
    pub async fn get_command_recommendations(
        &self,
        working_directory: &str,
        limit: Option<i64>,
    ) -> AppResult<Vec<(String, i64, f64)>> {
        let mut sql = String::from(
            r#"
            SELECT
                command,
                COUNT(*) as usage_count,
                AVG(CASE WHEN exit_code = 0 THEN 1.0 ELSE 0.0 END) as success_rate
            FROM command_history
            WHERE working_directory = ?
            GROUP BY command
            HAVING usage_count >= 2  -- 至少使用过2次
            ORDER BY usage_count DESC, success_rate DESC
        "#,
        );

        if let Some(limit_val) = limit {
            sql.push_str(&format!(" LIMIT {}", limit_val));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql).bind(working_directory))
            .await
            .map_err(|e| anyhow!("查询命令推荐失败: {}", e))?;

        let mut recommendations = Vec::new();
        for row in rows {
            let command: String = row.get("command");
            let usage_count: i64 = row.get("usage_count");
            let success_rate: f64 = row.get("success_rate");
            recommendations.push((command, usage_count, success_rate));
        }

        Ok(recommendations)
    }

    /// 批量保存命令历史（用于性能优化）
    pub async fn batch_save_command_history(
        &self,
        entries: &[CommandHistoryEntry],
    ) -> AppResult<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self
            .db_pool
            .begin()
            .await
            .map_err(|e| anyhow!("开始批量保存事务失败: {}", e))?;

        let sql = r#"
            INSERT INTO command_history
            (command, working_directory, exit_code, output, duration_ms, executed_at, session_id, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        for entry in entries {
            tx.execute(
                sqlx::query(sql)
                    .bind(&entry.command)
                    .bind(&entry.working_directory)
                    .bind(entry.exit_code)
                    .bind(&entry.output)
                    .bind(entry.duration_ms)
                    .bind(entry.executed_at)
                    .bind(&entry.session_id)
                    .bind(&entry.tags),
            )
            .await
            .map_err(|e| anyhow!("批量保存命令历史失败: {}", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| anyhow!("提交批量保存事务失败: {}", e))?;

        info!("批量保存了 {} 条命令历史记录", entries.len());
        Ok(())
    }
}
