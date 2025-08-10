/*!
 * SQLite数据管理器模块
 *
 * 管理长期数据存储、AI配置和智能查询
 * 实现数据库初始化、迁移系统、加密存储和智能查询功能
 */

use crate::ai::types::{AIModelConfig, Conversation, Message};
use crate::storage::paths::StoragePaths;
use crate::storage::sql_scripts::SqlScriptLoader;
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

/// SQLite数据管理器
pub struct SqliteManager {
    db_pool: SqlitePool,
    #[allow(dead_code)]
    paths: StoragePaths,
    #[allow(dead_code)]
    options: SqliteOptions,
    encryption_manager: Arc<RwLock<EncryptionManager>>,
    cache: Arc<RwLock<LruCache<String, Value>>>,
    sql_script_loader: SqlScriptLoader,
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

        // 初始化SQL脚本加载器，sql目录在src-tauri目录下
        let sql_dir = if cfg!(debug_assertions) {
            // 开发环境：固定使用crate根目录（src-tauri）下的 sql 目录，避免当前工作目录不一致的问题
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
        } else {
            // 生产环境：使用相对于可执行文件的sql目录
            std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("sql")
        };
        let sql_script_loader = SqlScriptLoader::new(sql_dir);

        let manager = Self {
            db_pool,
            paths,
            options,
            encryption_manager: Arc::new(RwLock::new(EncryptionManager::new())),
            cache,
            sql_script_loader,
        };

        Ok(manager)
    }

    /// 初始化数据库（重构版本：使用SQL脚本）
    pub async fn initialize_database(&self) -> AppResult<()> {
        info!("初始化SQLite数据库（使用SQL脚本）");

        // 检查sql目录是否存在
        let sql_dir = if cfg!(debug_assertions) {
            // 开发环境：固定使用crate根目录（src-tauri）下的 sql 目录
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
        } else {
            std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("sql")
        };

        if !sql_dir.exists() {
            return Err(anyhow!(
                "SQL脚本目录不存在: {}。请确保SQL脚本文件已正确部署。",
                sql_dir.display()
            ));
        }

        info!("使用SQL脚本目录: {}", sql_dir.display());

        // 设置默认主密钥（用于加密敏感数据）
        self.set_default_master_key()
            .await
            .context("设置默认主密钥失败")?;

        // 按顺序执行SQL脚本
        self.execute_sql_scripts()
            .await
            .context("执行SQL脚本失败")?;

        // 插入默认数据
        self.insert_default_data()
            .await
            .context("插入默认数据失败")?;

        info!("数据库初始化完成");
        Ok(())
    }

    /// 按顺序执行所有SQL脚本
    async fn execute_sql_scripts(&self) -> AppResult<()> {
        info!("开始执行SQL脚本");

        // 加载所有SQL脚本文件
        let scripts = self
            .sql_script_loader
            .load_all_scripts()
            .await
            .context("加载SQL脚本文件失败")?;

        if scripts.is_empty() {
            return Err(anyhow!("没有找到任何SQL脚本文件"));
        }

        info!("找到 {} 个SQL脚本文件", scripts.len());

        // 按执行顺序执行脚本
        for script in scripts {
            info!("执行SQL脚本: {} (顺序 {})", script.name, script.order);

            for (i, statement) in script.statements.iter().enumerate() {
                if !statement.is_empty() {
                    debug!(
                        "执行SQL语句 {}/{} ({}): {}",
                        i + 1,
                        script.statements.len(),
                        script.name,
                        statement
                    );

                    sqlx::query(statement)
                        .execute(self.pool())
                        .await
                        .map_err(|e| {
                            error!(
                                "SQL语句执行失败 {}/{} ({}): {} - SQL: {}",
                                i + 1,
                                script.statements.len(),
                                script.name,
                                e,
                                statement
                            );
                            anyhow!(
                                "SQL语句执行失败 {}/{} ({}): {} - SQL: {}",
                                i + 1,
                                script.statements.len(),
                                script.name,
                                e,
                                statement
                            )
                        })?;
                }
            }

            info!("SQL脚本 {} 执行完成", script.name);
        }

        info!("所有SQL脚本执行完成");
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

        // 安全的密钥获取策略：优先级从高到低
        let master_password = self.get_secure_master_key().await?;
        encryption_manager.set_master_password(&master_password)?;

        info!("主密钥设置完成");
        Ok(())
    }

    /// 安全获取主密钥
    /// 优先级：环境变量 > 系统密钥链 > 用户配置 > 安全随机生成
    async fn get_secure_master_key(&self) -> AppResult<String> {
        // 1. 尝试从环境变量获取
        if let Ok(key) = std::env::var("OrbitX_MASTER_KEY") {
            if !key.is_empty() && key.len() >= 16 {
                debug!("从环境变量获取主密钥");
                return Ok(key);
            } else {
                warn!("环境变量 OrbitX_MASTER_KEY 长度不足（需要至少16个字符）");
            }
        }

        // 2. 尝试从系统密钥链获取（仅在支持的平台上）
        if let Some(key) = self.get_key_from_system_keychain().await? {
            debug!("从系统密钥链获取主密钥");
            return Ok(key);
        }

        // 3. 尝试从用户配置目录的安全文件获取
        if let Some(key) = self.get_key_from_config_file().await? {
            debug!("从配置文件获取主密钥");
            return Ok(key);
        }

        // 4. 生成新的安全随机密钥并保存
        let new_key = self.generate_and_save_master_key().await?;
        info!("生成新的主密钥并保存到配置文件");
        Ok(new_key)
    }

    /// 从系统密钥链获取密钥
    async fn get_key_from_system_keychain(&self) -> AppResult<Option<String>> {
        // 在实际实现中，这里应该调用系统特定的密钥链API
        // 例如：macOS Keychain、Windows DPAPI、Linux Secret Service

        #[cfg(target_os = "macos")]
        {
            // 示例：macOS Keychain集成（需要security-framework crate）
            // let service = "terminal-app";
            // let account = "master-encryption-key";
            // 实际实现需要添加对应的依赖和代码
        }

        #[cfg(target_os = "windows")]
        {
            // 示例：Windows DPAPI集成（需要winapi crate）
            // 实际实现需要添加对应的依赖和代码
        }

        #[cfg(target_os = "linux")]
        {
            // 示例：Linux Secret Service集成（需要secret-service crate）
            // 实际实现需要添加对应的依赖和代码
        }

        // 当前简化实现：返回None，表示不支持系统密钥链
        debug!("系统密钥链支持尚未实现");
        Ok(None)
    }

    /// 从配置文件获取密钥
    async fn get_key_from_config_file(&self) -> AppResult<Option<String>> {
        let config_dir = self.paths.config_dir.clone();
        let key_file_path = config_dir.join(".master_key");

        if !key_file_path.exists() {
            return Ok(None);
        }

        match tokio::fs::read_to_string(&key_file_path).await {
            Ok(content) => {
                let key = content.trim().to_string();
                if key.len() >= 16 {
                    Ok(Some(key))
                } else {
                    warn!("配置文件中的密钥长度不足");
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("读取密钥配置文件失败: {}", e);
                Ok(None)
            }
        }
    }

    /// 生成并保存新的主密钥
    async fn generate_and_save_master_key(&self) -> AppResult<String> {
        use rand::{distributions::Alphanumeric, Rng};

        // 生成64字符的随机密钥
        let new_key: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        // 保存到配置文件
        let config_dir = &self.paths.config_dir;
        tokio::fs::create_dir_all(config_dir)
            .await
            .with_context(|| format!("创建配置目录失败: {}", config_dir.display()))?;

        let key_file_path = config_dir.join(".master_key");
        tokio::fs::write(&key_file_path, &new_key)
            .await
            .with_context(|| format!("保存主密钥文件失败: {}", key_file_path.display()))?;

        // 设置文件权限（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&key_file_path).await?.permissions();
            perms.set_mode(0o600); // rw-------
            tokio::fs::set_permissions(&key_file_path, perms).await?;
        }

        info!("新主密钥已生成并保存到: {}", key_file_path.display());
        Ok(new_key)
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

        // 构建SQL查询（安全版）：
        // - 仅允许 ORDER BY 的字段为白名单
        // - LIMIT/OFFSET 使用参数绑定（i64）
        let mut sql = query.query.clone();

        // 允许排序的字段白名单（按需扩展或通过配置注入）
        let allowed_order_fields = ["id", "created_at", "updated_at"]; // 示例
        if let Some(order_by) = &query.order_by {
            if allowed_order_fields.contains(&order_by.as_str()) {
                sql.push_str(&format!(
                    " ORDER BY {} {}",
                    order_by,
                    if query.desc { "DESC" } else { "ASC" }
                ));
            } else {
                debug!("忽略不在白名单中的 ORDER BY 字段: {}", order_by);
            }
        }

        let limit_i64: Option<i64> = query.limit.map(|v| v as i64);
        let offset_i64: Option<i64> = query.offset.map(|v| v as i64);
        if limit_i64.is_some() {
            sql.push_str(" LIMIT ?");
        }
        if offset_i64.is_some() {
            sql.push_str(" OFFSET ?");
        }

        let mut q = sqlx::query(&sql);
        if let Some(l) = limit_i64 {
            q = q.bind(l);
        }
        if let Some(o) = offset_i64 {
            q = q.bind(o);
        }

        // 执行查询
        let rows = self
            .db_pool
            .fetch_all(q)
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

        // 清除相关缓存（AI相关表除外）
        if !table.starts_with("ai_") {
            self.clear_table_cache(table).await;
        }

        // 根据表名选择保存策略
        match table.as_str() {
            "ai_models" => self.save_ai_model_data(data, options).await,
            "command_history" => self.save_command_history_data(data, options).await,

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

        Ok(())
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

        // 参数化绑定，避免注入
        if table_name.is_some() {
            sql.push_str(" AND table_name = ?");
        }
        if operation.is_some() {
            sql.push_str(" AND operation = ?");
        }
        sql.push_str(" ORDER BY timestamp DESC");
        if limit.is_some() {
            sql.push_str(" LIMIT ?");
        }

        let mut q = sqlx::query(&sql);
        if let Some(table) = table_name {
            q = q.bind(table);
        }
        if let Some(op) = operation {
            q = q.bind(op);
        }
        if let Some(limit_val) = limit {
            q = q.bind(limit_val);
        }

        let rows = self
            .db_pool
            .fetch_all(q)
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

        // 参数化查询，避免注入
        let mut args: Vec<(String, String)> = Vec::new();
        if let Some(pattern) = &query.command_pattern {
            sql.push_str(" AND command LIKE ?");
            args.push(("like".to_string(), format!("%{}%", pattern)));
        }

        if let Some(working_dir) = &query.working_directory {
            sql.push_str(" AND working_directory = ?");
            args.push(("eq".to_string(), working_dir.clone()));
        }

        // session_id 字段已从 HistoryQuery 中移除

        if let Some(date_from) = &query.date_from {
            sql.push_str(" AND executed_at >= ?");
            args.push((
                "ge".to_string(),
                date_from.format("%Y-%m-%d %H:%M:%S").to_string(),
            ));
        }

        if let Some(date_to) = &query.date_to {
            sql.push_str(" AND executed_at <= ?");
            args.push((
                "le".to_string(),
                date_to.format("%Y-%m-%d %H:%M:%S").to_string(),
            ));
        }

        sql.push_str(" ORDER BY executed_at DESC");

        // LIMIT/OFFSET 参数化 + 执行
        let limit_i64: Option<i64> = query.limit.map(|v| v as i64);
        let offset_i64: Option<i64> = query.offset.map(|v| v as i64);
        if limit_i64.is_some() {
            sql.push_str(" LIMIT ?");
        }
        if offset_i64.is_some() {
            sql.push_str(" OFFSET ?");
        }

        let mut q = sqlx::query(&sql);
        for (_k, v) in &args {
            q = q.bind(v);
        }
        if let Some(l) = limit_i64 {
            q = q.bind(l);
        }
        if let Some(o) = offset_i64 {
            q = q.bind(o);
        }
        let rows = self
            .db_pool
            .fetch_all(q)
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

    // ===== AI会话上下文管理系统 - 全新方法 =====

    /// 创建新会话
    pub async fn create_conversation(&self, conversation: &Conversation) -> AppResult<i64> {
        debug!("创建会话: title={}", conversation.title);

        let sql = r#"
            INSERT INTO ai_conversations (title, message_count, last_message_preview, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
        "#;

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(&conversation.title)
                    .bind(conversation.message_count)
                    .bind(&conversation.last_message_preview)
                    .bind(conversation.created_at)
                    .bind(conversation.updated_at),
            )
            .await
            .with_context(|| "创建会话失败")?;

        Ok(result.last_insert_rowid())
    }

    /// 获取会话列表
    pub async fn get_conversations(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<Vec<Conversation>> {
        debug!("查询会话列表: limit={:?}, offset={:?}", limit, offset);

        let mut sql = String::from(
            r#"
            SELECT id, title, message_count, last_message_preview, created_at, updated_at
            FROM ai_conversations
            ORDER BY updated_at DESC
        "#,
        );

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql))
            .await
            .with_context(|| "查询会话列表失败")?;

        let conversations: Vec<Conversation> = rows
            .iter()
            .map(|row| self.row_to_conversation(row))
            .collect();

        Ok(conversations)
    }

    /// 获取单个会话
    pub async fn get_conversation(&self, conversation_id: i64) -> AppResult<Option<Conversation>> {
        debug!("查询会话: {}", conversation_id);

        let sql = r#"
            SELECT id, title, message_count, last_message_preview, created_at, updated_at
            FROM ai_conversations
            WHERE id = ?
        "#;

        let row = self
            .db_pool
            .fetch_optional(sqlx::query(sql).bind(conversation_id))
            .await
            .with_context(|| format!("查询会话失败: {}", conversation_id))?;

        Ok(row.map(|r| self.row_to_conversation(&r)))
    }

    /// 更新会话标题
    pub async fn update_conversation_title(
        &self,
        conversation_id: i64,
        title: &str,
    ) -> AppResult<()> {
        debug!("更新会话标题: {} -> {}", conversation_id, title);

        let sql = r#"
            UPDATE ai_conversations
            SET title = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
        "#;

        self.db_pool
            .execute(sqlx::query(sql).bind(title).bind(conversation_id))
            .await
            .with_context(|| format!("更新会话标题失败: {}", conversation_id))?;

        Ok(())
    }

    /// 删除会话
    pub async fn delete_conversation(&self, conversation_id: i64) -> AppResult<()> {
        debug!("删除会话: {}", conversation_id);

        // 由于设置了级联删除，删除会话会自动删除相关消息
        let sql = "DELETE FROM ai_conversations WHERE id = ?";

        let result = self
            .db_pool
            .execute(sqlx::query(sql).bind(conversation_id))
            .await
            .with_context(|| format!("删除会话失败: {}", conversation_id))?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("会话不存在: {}", conversation_id));
        }

        Ok(())
    }

    /// 保存消息
    pub async fn save_message(&self, message: &Message) -> AppResult<i64> {
        debug!(
            "保存消息: conversation_id={}, role={}",
            message.conversation_id, message.role
        );

        let sql = r#"
            INSERT INTO ai_messages (conversation_id, role, content, created_at)
            VALUES (?, ?, ?, ?)
        "#;

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(message.conversation_id)
                    .bind(&message.role)
                    .bind(&message.content)
                    .bind(message.created_at),
            )
            .await
            .with_context(|| "保存消息失败")?;

        Ok(result.last_insert_rowid())
    }

    /// 获取会话消息
    pub async fn get_messages(
        &self,
        conversation_id: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<Vec<Message>> {
        debug!(
            "查询消息: conversation_id={}, limit={:?}, offset={:?}",
            conversation_id, limit, offset
        );

        let mut sql = String::from(
            r#"
            SELECT id, conversation_id, role, content, created_at
            FROM ai_messages
            WHERE conversation_id = ?
            ORDER BY created_at ASC
        "#,
        );

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = self
            .db_pool
            .fetch_all(sqlx::query(&sql).bind(conversation_id))
            .await
            .with_context(|| format!("查询消息失败: {}", conversation_id))?;

        let messages: Vec<Message> = rows.iter().map(|row| self.row_to_message(row)).collect();

        Ok(messages)
    }

    /// 获取指定位置之前的消息（用于截断重问）
    pub async fn get_messages_up_to(
        &self,
        conversation_id: i64,
        up_to_message_id: i64,
    ) -> AppResult<Vec<Message>> {
        debug!(
            "查询截断消息: conversation_id={}, up_to={}",
            conversation_id, up_to_message_id
        );

        let sql = r#"
            SELECT id, conversation_id, role, content, created_at
            FROM ai_messages
            WHERE conversation_id = ? AND id <= ?
            ORDER BY created_at ASC
        "#;

        let rows = self
            .db_pool
            .fetch_all(
                sqlx::query(sql)
                    .bind(conversation_id)
                    .bind(up_to_message_id),
            )
            .await
            .with_context(|| "查询截断消息失败")?;

        let messages: Vec<Message> = rows.iter().map(|row| self.row_to_message(row)).collect();

        Ok(messages)
    }

    /// 删除指定消息ID之后的所有消息（用于截断重问）
    pub async fn delete_messages_after(
        &self,
        conversation_id: i64,
        after_message_id: i64,
    ) -> AppResult<u64> {
        debug!(
            "删除截断消息: conversation_id={}, after={}",
            conversation_id, after_message_id
        );

        let sql = r#"
            DELETE FROM ai_messages
            WHERE conversation_id = ? AND id > ?
        "#;

        let result = self
            .db_pool
            .execute(
                sqlx::query(sql)
                    .bind(conversation_id)
                    .bind(after_message_id),
            )
            .await
            .with_context(|| "删除消息失败")?;

        Ok(result.rows_affected())
    }

    /// 获取会话的最后一条消息
    pub async fn get_last_message(&self, conversation_id: i64) -> AppResult<Option<Message>> {
        debug!("查询最后消息: conversation_id={}", conversation_id);

        let sql = r#"
            SELECT id, conversation_id, role, content, created_at
            FROM ai_messages
            WHERE conversation_id = ?
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let row = self
            .db_pool
            .fetch_optional(sqlx::query(sql).bind(conversation_id))
            .await
            .with_context(|| format!("查询最后消息失败: {}", conversation_id))?;

        Ok(row.map(|r| self.row_to_message(&r)))
    }

    /// 更新会话预览
    pub async fn update_conversation_preview(
        &self,
        conversation_id: i64,
        preview: &str,
    ) -> AppResult<()> {
        debug!("更新会话预览: {} -> {}", conversation_id, preview);

        let sql = r#"
            UPDATE ai_conversations
            SET last_message_preview = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
        "#;

        self.db_pool
            .execute(sqlx::query(sql).bind(preview).bind(conversation_id))
            .await
            .with_context(|| format!("更新会话预览失败: {}", conversation_id))?;

        Ok(())
    }

    /// 数据库行转换为会话对象
    fn row_to_conversation(&self, row: &SqliteRow) -> Conversation {
        Conversation {
            id: row.get("id"),
            title: row.get("title"),
            message_count: row.get("message_count"),
            last_message_preview: row.get("last_message_preview"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// 数据库行转换为消息对象
    fn row_to_message(&self, row: &SqliteRow) -> Message {
        Message {
            id: row.get("id"),
            conversation_id: row.get("conversation_id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at: row.get("created_at"),
        }
    }
}
