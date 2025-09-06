/*!
 * 数据库管理器模块
 *
 * 提供SQLite数据库的核心管理功能，包括连接池、初始化、加密等
 */

use crate::storage::paths::StoragePaths;
use crate::storage::sql_scripts::SqlScriptLoader;
use crate::storage::DATABASE_FILE_NAME;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        SaltString,
    },
    Argon2, PasswordHasher,
};
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng as ChaChaOsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use sqlx::{
    sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
    },
    ConnectOptions, Executor,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 数据库管理器选项
#[derive(Debug, Clone)]
pub struct DatabaseOptions {
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
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            encryption: true,
            pool_size: 10,
            connection_timeout: 30,
            query_timeout: 30,
            wal_mode: true,
        }
    }
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
        // 生成随机盐值而不是使用固定盐值
        let salt = SaltString::generate(&mut OsRng);

        let password_hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("密钥派生失败: {}", e))?;

        let hash = password_hash.hash.unwrap();
        let key_bytes = hash.as_bytes();
        if key_bytes.len() < 32 {
            return Err(anyhow!("密钥长度不足"));
        }

        self.master_key = Some(*Key::from_slice(&key_bytes[..32]));
        Ok(())
    }

    /// 加密敏感数据
    pub fn encrypt_data(&self, data: &str) -> AppResult<Vec<u8>> {
        let key = self
            .master_key
            .as_ref()
            .ok_or_else(|| anyhow!("未设置主密钥"))?;

        let cipher = ChaCha20Poly1305::new(key);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut ChaChaOsRng);

        let ciphertext = cipher
            .encrypt(&nonce, data.as_bytes())
            .map_err(|e| anyhow!("加密失败: {}", e))?;

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
            .ok_or_else(|| anyhow!("未设置主密钥"))?;

        if encrypted_data.len() < 12 {
            return Err(anyhow!("加密数据格式错误"));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new(key);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("解密失败: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow!("解密数据格式错误: {}", e))
    }
}

/// 数据库管理器
pub struct DatabaseManager {
    db_pool: SqlitePool,
    paths: StoragePaths,
    encryption_manager: Arc<RwLock<EncryptionManager>>,
    sql_script_loader: SqlScriptLoader,
}

impl DatabaseManager {
    /// 创建新的数据库管理器
    pub async fn new(paths: StoragePaths, options: DatabaseOptions) -> AppResult<Self> {
        let db_path = paths.data_dir.join(DATABASE_FILE_NAME);

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

        // 初始化SQL脚本加载器
        let sql_dir = if cfg!(debug_assertions) {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
        } else {
            let exe_path =
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("."));

            if let Some(contents_dir) = exe_path
                .ancestors()
                .find(|p| p.file_name() == Some(std::ffi::OsStr::new("Contents")))
            {
                contents_dir.join("Resources").join("sql")
            } else {
                exe_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join("sql")
            }
        };
        let sql_script_loader = SqlScriptLoader::new(sql_dir);

        let manager = Self {
            db_pool,
            paths,
            encryption_manager: Arc::new(RwLock::new(EncryptionManager::new())),
            sql_script_loader,
        };

        Ok(manager)
    }

    /// 初始化数据库
    pub async fn initialize(&self) -> AppResult<()> {
        info!("初始化SQLite数据库");

        // 设置默认主密钥
        self.set_default_master_key().await?;

        // 执行SQL脚本
        self.execute_sql_scripts().await?;

        // 插入默认数据
        self.insert_default_data().await?;

        info!("数据库初始化完成");
        Ok(())
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    /// 获取加密管理器
    pub fn encryption_manager(&self) -> Arc<RwLock<EncryptionManager>> {
        self.encryption_manager.clone()
    }

    /// 设置主密钥
    pub async fn set_master_password(&self, password: &str) -> AppResult<()> {
        let mut encryption_manager = self.encryption_manager.write().await;
        encryption_manager.set_master_password(password)?;
        info!("主密钥设置成功");
        Ok(())
    }

    /// 设置默认主密钥
    async fn set_default_master_key(&self) -> AppResult<()> {
        let mut encryption_manager = self.encryption_manager.write().await;

        if encryption_manager.master_key.is_some() {
            debug!("主密钥已设置，跳过默认密钥设置");
            return Ok(());
        }

        let master_password = self.get_secure_master_key().await?;
        encryption_manager.set_master_password(&master_password)?;

        info!("主密钥设置完成");
        Ok(())
    }

    /// 获取安全的主密钥
    async fn get_secure_master_key(&self) -> AppResult<String> {
        match self.get_key_from_config_file().await {
            Ok(Some(key)) => {
                debug!("从配置文件获取主密钥");
                return Ok(key);
            }
            Ok(None) => {
                debug!("配置文件中未找到主密钥");
            }
            Err(e) => {
                warn!("从配置文件读取主密钥失败: {}，将生成新的主密钥", e);
            }
        }

        let new_key = self.generate_deterministic_master_key().await?;
        info!("生成用户机器密钥并保存到配置文件");
        Ok(new_key)
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

    /// 生成基于机器标识的安全主密钥
    async fn generate_deterministic_master_key(&self) -> AppResult<String> {
        use sha2::{Digest, Sha256};

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown_user".to_string());

        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| {
                std::process::Command::new("hostname")
                    .output()
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown_host".to_string())
            });

        let home_dir = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown_home".to_string());

        // 添加随机数据增强安全性
        let mut random_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut random_bytes);

        let mut hasher = Sha256::default();
        hasher.update(b"OrbitX-Terminal-App-v1.0");
        hasher.update(username.as_bytes());
        hasher.update(hostname.as_bytes());
        hasher.update(home_dir.as_bytes());
        hasher.update(b"encryption-key-salt");
        // 添加随机因子增强安全性
        hasher.update(&random_bytes);

        let hash = hasher.finalize();
        let key = base64::engine::general_purpose::STANDARD.encode(hash);

        // 保存到配置文件
        let config_dir = &self.paths.config_dir;
        tokio::fs::create_dir_all(config_dir)
            .await
            .with_context(|| format!("创建配置目录失败: {}", config_dir.display()))?;

        let key_file_path = config_dir.join(".master_key");
        tokio::fs::write(&key_file_path, &key)
            .await
            .with_context(|| format!("保存主密钥文件失败: {}", key_file_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&key_file_path).await?.permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&key_file_path, perms).await?;
        }

        info!("机器标识密钥已生成并保存到: {}", key_file_path.display());
        debug!("基于用户: {}, 主机: {}", username, hostname);
        Ok(key)
    }

    /// 执行SQL脚本
    async fn execute_sql_scripts(&self) -> AppResult<()> {
        info!("开始执行SQL脚本");

        let scripts = self
            .sql_script_loader
            .load_all_scripts()
            .await
            .context("加载SQL脚本文件失败")?;

        if scripts.is_empty() {
            return Err(anyhow!("没有找到任何SQL脚本文件"));
        }

        info!("找到 {} 个SQL脚本文件", scripts.len());

        for script in scripts {
            info!("执行SQL脚本: {} (顺序 {})", script.name, script.order);

            for (i, statement) in script.statements.iter().enumerate() {
                if !statement.is_empty() {
                    sqlx::query(statement)
                        .execute(&self.db_pool)
                        .await
                        .map_err(|e| {
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

    /// 插入默认数据
    async fn insert_default_data(&self) -> AppResult<()> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_manager_creation() {
        let manager = EncryptionManager::new();
        assert!(manager.master_key.is_none());
    }

    #[test]
    fn test_set_master_password_random_salt() {
        let mut manager1 = EncryptionManager::new();
        let mut manager2 = EncryptionManager::new();

        // 使用相同密码设置主密钥
        let password = "test_password_123";
        let result1 = manager1.set_master_password(password);
        let result2 = manager2.set_master_password(password);

        // 两次操作都应该成功
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // 两个管理器都应该有主密钥
        assert!(manager1.master_key.is_some());
        assert!(manager2.master_key.is_some());

        // 由于使用随机盐值，每次生成的密钥应该不同（尽管密码相同）
        // 注意：理论上有极小概率相同，但实际上几乎不可能
        let key1 = manager1.master_key.unwrap();
        let key2 = manager2.master_key.unwrap();
        assert_ne!(key1, key2, "随机盐值应该产生不同的密钥");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut manager = EncryptionManager::new();
        let password = "test_password_123";
        manager
            .set_master_password(password)
            .expect("设置主密钥失败");

        let test_data = "这是需要加密的敏感数据";

        // 加密数据
        let encrypted = manager.encrypt_data(test_data).expect("加密失败");
        assert!(!encrypted.is_empty());

        // 解密数据
        let decrypted = manager.decrypt_data(&encrypted).expect("解密失败");
        assert_eq!(decrypted, test_data);
    }

    #[test]
    fn test_encrypt_without_master_key_fails() {
        let manager = EncryptionManager::new();
        let test_data = "测试数据";

        let result = manager.encrypt_data(test_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未设置主密钥"));
    }

    #[test]
    fn test_decrypt_without_master_key_fails() {
        let manager = EncryptionManager::new();
        let test_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let result = manager.decrypt_data(&test_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未设置主密钥"));
    }

    #[test]
    fn test_decrypt_invalid_data_fails() {
        let mut manager = EncryptionManager::new();
        manager.set_master_password("test").expect("设置主密钥失败");

        // 测试数据太短
        let invalid_data = vec![1, 2, 3];
        let result = manager.decrypt_data(&invalid_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("加密数据格式错误"));
    }

    #[test]
    fn test_multiple_encryptions_produce_different_ciphertexts() {
        let mut manager = EncryptionManager::new();
        manager.set_master_password("test").expect("设置主密钥失败");

        let test_data = "相同的明文数据";

        // 多次加密相同数据
        let encrypted1 = manager.encrypt_data(test_data).expect("第一次加密失败");
        let encrypted2 = manager.encrypt_data(test_data).expect("第二次加密失败");

        // 由于使用不同的随机 nonce，密文应该不同
        assert_ne!(encrypted1, encrypted2, "相同明文的多次加密应该产生不同密文");

        // 但解密结果应该相同
        let decrypted1 = manager.decrypt_data(&encrypted1).expect("第一次解密失败");
        let decrypted2 = manager.decrypt_data(&encrypted2).expect("第二次解密失败");
        assert_eq!(decrypted1, test_data);
        assert_eq!(decrypted2, test_data);
    }
}
