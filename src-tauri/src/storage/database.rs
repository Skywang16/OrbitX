use crate::storage::paths::StoragePaths;
use crate::storage::sql_scripts::{SqlScript, SqlScriptCatalog};
use crate::storage::DATABASE_FILE_NAME;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{ConnectOptions, Executor};
use std::fmt;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

const KEY_FILE_NAME: &str = "master.key";
const KEY_FILE_VERSION: &str = "v1";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone)]
pub enum PoolSize {
    Fixed(NonZeroU32),
    Adaptive { min: NonZeroU32, max: NonZeroU32 },
}

impl PoolSize {
    fn resolve(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            PoolSize::Fixed(size) => (*size, *size),
            PoolSize::Adaptive { min, max } => {
                let cpu = std::thread::available_parallelism()
                    .map(|n| n.get() as u32)
                    .unwrap_or(4);
                let suggested = (cpu * 2).clamp(min.get(), max.get());
                (*min, NonZeroU32::new(suggested).unwrap())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseOptions {
    pub encryption: bool,
    pub pool_size: PoolSize,
    pub connection_timeout: Duration,
    pub statement_timeout: Duration,
    pub wal: bool,
    pub sql_dir: Option<PathBuf>,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            encryption: true,
            pool_size: PoolSize::Adaptive {
                min: NonZeroU32::new(4).unwrap(),
                max: NonZeroU32::new(32).unwrap(),
            },
            connection_timeout: Duration::from_secs(10),
            statement_timeout: Duration::from_secs(30),
            wal: true,
            sql_dir: None,
        }
    }
}

pub struct DatabaseManager {
    pool: SqlitePool,
    paths: StoragePaths,
    options: DatabaseOptions,
    scripts: Arc<[SqlScript]>,
    key_vault: Arc<KeyVault>,
}

impl fmt::Debug for DatabaseManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseManager")
            .field("paths", &self.paths)
            .field("options", &self.options)
            .field("script_count", &self.scripts.len())
            .finish()
    }
}

impl DatabaseManager {
    pub async fn new(paths: StoragePaths, options: DatabaseOptions) -> AppResult<Self> {
        let db_path = paths.data_dir.join(DATABASE_FILE_NAME);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建数据库目录失败: {}", parent.display()))?;
        }

        let (min_conn, max_conn) = options.pool_size.resolve();

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(if options.wal {
                SqliteJournalMode::Wal
            } else {
                SqliteJournalMode::Delete
            })
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(options.statement_timeout)
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .min_connections(min_conn.get())
            .max_connections(max_conn.get())
            .acquire_timeout(options.connection_timeout)
            .idle_timeout(Some(Duration::from_secs(30)))
            .max_lifetime(Some(Duration::from_secs(60 * 15)))
            .connect_with(connect_options)
            .await
            .with_context(|| format!("连接SQLite失败: {}", db_path.display()))?;

        let sql_dir = resolve_sql_dir(&options);
        let scripts = SqlScriptCatalog::load(sql_dir)
            .await
            .context("加载SQL脚本失败")?
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into();

        let key_vault = Arc::new(KeyVault::new(paths.config_dir.join(KEY_FILE_NAME)));

        Ok(Self {
            pool,
            paths,
            options,
            scripts,
            key_vault,
        })
    }

    pub async fn initialize(&self) -> AppResult<()> {
        if self.options.encryption {
            self.key_vault.master_key().await?;
        }

        self.pool
            .execute("PRAGMA foreign_keys = ON")
            .await
            .context("启用外键失败")?;

        if self.options.encryption {
            self.pool
                .execute("PRAGMA secure_delete = ON")
                .await
                .context("启用secure_delete失败")?;
        }

        self.execute_sql_scripts().await?;
        self.insert_default_data().await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn set_master_password(&self, password: &str) -> AppResult<()> {
        if !self.options.encryption {
            return Err(anyhow!("Encryption not enabled"));
        }
        self.key_vault.set_from_password(password).await?;
        info!("主密钥已更新");
        Ok(())
    }

    pub async fn encrypt_data(&self, data: &str) -> AppResult<Vec<u8>> {
        if !self.options.encryption {
            return Err(anyhow!("Encryption not enabled"));
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(|e| anyhow!("加密失败: {}", e))?;
        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub async fn decrypt_data(&self, encrypted: &[u8]) -> AppResult<String> {
        if !self.options.encryption {
            return Err(anyhow!("Encryption not enabled"));
        }
        if encrypted.len() <= NONCE_LEN {
            return Err(anyhow!("Invalid encrypted data format"));
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let (nonce_bytes, payload) = encrypted.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(|e| anyhow!("解密失败: {}", e))?;
        String::from_utf8(plaintext).map_err(|e| anyhow!("解密数据不是UTF-8: {}", e))
    }

    async fn execute_sql_scripts(&self) -> AppResult<()> {
        if self.scripts.is_empty() {
            debug!("SQL脚本目录为空，跳过初始化");
            return Ok(());
        }

        for script in self.scripts.iter() {
            debug!("执行SQL脚本: {}", script.name);
            for statement in script.statements.iter() {
                if statement.trim().is_empty() {
                    continue;
                }
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await
                    .with_context(|| format!("执行SQL失败: {}", statement))?;
            }
        }

        Ok(())
    }

    async fn insert_default_data(&self) -> AppResult<()> {
        let features = [
            ("chat", true, r#"{"max_history":100,"auto_save":true}"#),
            ("explanation", true, r#"{"auto_explain":false}"#),
            ("command_search", true, r#"{"max_results":50}"#),
        ];

        for (feature_name, enabled, config_json) in features {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO ai_features (feature_name, enabled, config_json)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(feature_name)
            .bind(enabled)
            .bind(config_json)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("插入默认AI配置失败: {}", e))?;
        }

        Ok(())
    }
}

struct KeyVault {
    path: PathBuf,
    cache: RwLock<Option<[u8; 32]>>,
    argon2: Argon2<'static>,
}

impl KeyVault {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            cache: RwLock::new(None),
            argon2: Argon2::default(),
        }
    }

    async fn master_key(&self) -> AppResult<[u8; 32]> {
        if let Some(bytes) = *self.cache.read().await {
            return Ok(bytes);
        }

        let mut write_guard = self.cache.write().await;
        if let Some(bytes) = *write_guard {
            return Ok(bytes);
        }

        let bytes = if let Some(bytes) = self.load_from_disk().await? {
            bytes
        } else {
            self.generate_random().await?
        };

        *write_guard = Some(bytes);
        Ok(bytes)
    }

    async fn set_from_password(&self, password: &str) -> AppResult<[u8; 32]> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("密钥派生失败: {}", e))?;

        let hash = password_hash
            .hash
            .ok_or_else(|| anyhow!("密钥派生失败: 空哈希"))?;
        let hash_bytes = hash.as_bytes();
        if hash_bytes.len() < 32 {
            return Err(anyhow!("Insufficient key length"));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&hash_bytes[..32]);
        self.persist(bytes).await?;
        *self.cache.write().await = Some(bytes);
        Ok(bytes)
    }

    async fn load_from_disk(&self) -> AppResult<Option<[u8; 32]>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = tokio::fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("读取密钥文件失败: {}", self.path.display()))?;
        let mut lines = raw.lines();
        let first = lines.next().unwrap_or_default();
        let encoded = if first == KEY_FILE_VERSION {
            lines.next().unwrap_or_default()
        } else {
            first
        };
        if encoded.is_empty() {
            return Ok(None);
        }
        let decoded = BASE64
            .decode(encoded)
            .map_err(|e| anyhow!("Failed to parse key: {}", e))?;
        if decoded.len() != 32 {
            return Err(anyhow!("Invalid key length"));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Some(bytes))
    }

    async fn generate_random(&self) -> AppResult<[u8; 32]> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        self.persist(bytes).await?;
        Ok(bytes)
    }

    async fn persist(&self, bytes: [u8; 32]) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建密钥目录失败: {}", parent.display()))?;
        }
        let encoded = BASE64.encode(bytes);
        let payload = format!("{}\n{}\n", KEY_FILE_VERSION, encoded);
        let tmp_path = self.path.with_extension("tmp");
        tokio::fs::write(&tmp_path, payload.as_bytes())
            .await
            .with_context(|| format!("写入密钥临时文件失败: {}", tmp_path.display()))?;
        tokio::fs::rename(&tmp_path, &self.path)
            .await
            .with_context(|| format!("替换密钥文件失败: {}", self.path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&self.path).await?.permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&self.path, perms).await?;
        }

        Ok(())
    }
}

fn resolve_sql_dir(options: &DatabaseOptions) -> PathBuf {
    if let Some(custom) = &options.sql_dir {
        return custom.clone();
    }

    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
    } else {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        if let Some(contents) = exe
            .ancestors()
            .find(|p| p.file_name() == Some(std::ffi::OsStr::new("Contents")))
        {
            contents.join("Resources/sql")
        } else {
            exe.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("sql")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn key_vault_generates_and_persists() {
        let temp_dir = TempDir::new().unwrap();
        let vault = KeyVault::new(temp_dir.path().join(KEY_FILE_NAME));
        let key1 = vault.master_key().await.unwrap();
        let key2 = vault.master_key().await.unwrap();
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn key_vault_accepts_password() {
        let temp_dir = TempDir::new().unwrap();
        let vault = KeyVault::new(temp_dir.path().join(KEY_FILE_NAME));
        let key1 = vault.set_from_password("secret").await.unwrap();
        let key2 = vault.master_key().await.unwrap();
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn encryption_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let paths = crate::storage::paths::StoragePathsBuilder::new()
            .app_dir(temp_dir.path().to_path_buf())
            .build()
            .unwrap();
        paths.ensure_directories().unwrap();

        let manager = DatabaseManager::new(paths.clone(), DatabaseOptions::default())
            .await
            .unwrap();
        manager.initialize().await.unwrap();

        let encrypted = manager.encrypt_data("hello world").await.unwrap();
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        assert_eq!(decrypted, "hello world");
    }
}
