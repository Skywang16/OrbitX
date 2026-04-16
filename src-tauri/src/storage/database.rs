use crate::storage::error::{DatabaseError, DatabaseResult};
use crate::storage::paths::StoragePaths;
use crate::storage::sql_scripts::{SqlScript, SqlScriptCatalog};
use crate::storage::DATABASE_FILE_NAME;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::ChaCha20Poly1305;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{ConnectOptions, Executor, Row};
use std::fmt;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing;

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
                let cpu = match std::thread::available_parallelism() {
                    Ok(parallelism) => parallelism.get() as u32,
                    Err(err) => {
                        tracing::warn!(
                            "Failed to detect available parallelism, using default pool size: {}",
                            err
                        );
                        4
                    }
                };
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
    pub async fn new(paths: StoragePaths, options: DatabaseOptions) -> DatabaseResult<Self> {
        let db_path = paths.data_dir.join(DATABASE_FILE_NAME);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                DatabaseError::io(
                    format!("create database directory {}", parent.display()),
                    err,
                )
            })?;
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
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to connect SQLite: {} ({err})",
                    db_path.display()
                ))
            })?;

        let sql_dir = resolve_sql_dir(&options)?;
        let scripts = SqlScriptCatalog::load(sql_dir)
            .await
            .map_err(DatabaseError::from)?
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

    pub async fn initialize(&self) -> DatabaseResult<()> {
        if self.options.encryption {
            self.key_vault.master_key().await?;
        }

        self.pool
            .execute("PRAGMA foreign_keys = ON")
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to enable foreign_keys pragma: {err}"))
            })?;

        if self.options.encryption {
            self.pool
                .execute("PRAGMA secure_delete = ON")
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!("Failed to enable secure_delete pragma: {err}"))
                })?;
        }

        self.execute_sql_scripts().await?;
        self.ensure_ai_models_schema().await?;
        self.ensure_workspaces_schema().await?;
        self.ensure_threads_schema().await?;
        self.insert_default_data().await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn config_dir(&self) -> &Path {
        &self.paths.config_dir
    }

    pub fn state_dir(&self) -> &Path {
        &self.paths.state_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.paths.data_dir
    }

    pub async fn encrypt_data(&self, data: &str) -> DatabaseResult<Vec<u8>> {
        if !self.options.encryption {
            return Err(DatabaseError::EncryptionNotEnabled);
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(key_bytes.as_ref().into());
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = nonce_bytes.as_ref().into();
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(DatabaseError::from)?;
        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub async fn decrypt_data(&self, encrypted: &[u8]) -> DatabaseResult<String> {
        if !self.options.encryption {
            return Err(DatabaseError::EncryptionNotEnabled);
        }
        if encrypted.len() <= NONCE_LEN {
            return Err(DatabaseError::InvalidEncryptedData);
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(key_bytes.as_ref().into());
        let (nonce_bytes, payload) = encrypted.split_at(NONCE_LEN);
        let nonce = nonce_bytes.into();
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(DatabaseError::from)?;
        String::from_utf8(plaintext).map_err(DatabaseError::from)
    }

    async fn execute_sql_scripts(&self) -> DatabaseResult<()> {
        if self.scripts.is_empty() {
            tracing::warn!("No SQL scripts found for database initialization");
            return Ok(());
        }

        tracing::info!("Executing {} SQL scripts", self.scripts.len());
        for script in self.scripts.iter() {
            tracing::info!("Executing script: {}", script.name);
            for statement in script.statements.iter() {
                if statement.trim().is_empty() {
                    continue;
                }
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| {
                        DatabaseError::internal(format!(
                            "Failed to execute SQL statement in script {}: {err}\nStatement: {statement}",
                            script.name
                        ))
                    })?;
            }
        }

        tracing::info!("All SQL scripts executed successfully");
        Ok(())
    }

    async fn ensure_ai_models_schema(&self) -> DatabaseResult<()> {
        let indexes = sqlx::query("PRAGMA index_list(ai_models)")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to inspect ai_models schema: {err}"))
            })?;

        let mut has_provider_model_unique = false;
        for row in indexes {
            let is_unique = pragma_i64_column(&row, "unique", "ai_models index_list")?;
            if is_unique == 0 {
                continue;
            }
            let name = pragma_text_column(&row, "name", "ai_models index_list")?;
            let columns = sqlx::query(format!("PRAGMA index_info({name})").as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to inspect ai_models index {name}: {err}"
                    ))
                })?;
            let mut column_names = Vec::new();
            for col in columns {
                column_names.push(pragma_text_column(&col, "name", "ai_models index_info")?);
            }
            if column_names.len() == 2
                && column_names.contains(&"provider".to_string())
                && column_names.contains(&"model_name".to_string())
            {
                has_provider_model_unique = true;
                break;
            }
        }

        let columns = sqlx::query("PRAGMA table_info(ai_models)")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to inspect ai_models columns: {err}"))
            })?;

        let mut has_display_name = false;
        for row in &columns {
            if pragma_text_column(row, "name", "ai_models table_info")? == "display_name" {
                has_display_name = true;
                break;
            }
        }

        if !has_display_name {
            sqlx::query("ALTER TABLE ai_models ADD COLUMN display_name TEXT")
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to migrate ai_models schema (add display_name): {err}"
                    ))
                })?;

            sqlx::query("UPDATE ai_models SET display_name = model_name WHERE display_name IS NULL OR display_name = ''")
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to backfill ai_models display_name: {err}"
                    ))
                })?;
        }

        if !has_provider_model_unique {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(|err| {
            DatabaseError::internal(format!("Failed to begin transaction: {err}"))
        })?;

        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to disable foreign_keys during ai_models migration: {err}"
                ))
            })?;

        sqlx::query(
            r#"
            CREATE TABLE ai_models_new (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                api_url TEXT,
                api_key_encrypted TEXT,
                model_name TEXT NOT NULL,
                display_name TEXT,
                model_type TEXT DEFAULT 'chat' CHECK (model_type IN ('chat', 'embedding')),
                config_json TEXT,
                auth_type TEXT NOT NULL DEFAULT 'api_key' CHECK (auth_type IN ('api_key', 'oauth')),
                oauth_provider TEXT CHECK (oauth_provider IN ('openai_codex', 'claude_pro', 'gemini_advanced') OR oauth_provider IS NULL),
                oauth_refresh_token_encrypted TEXT,
                oauth_access_token_encrypted TEXT,
                oauth_token_expires_at INTEGER,
                oauth_metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            DatabaseError::internal(format!(
                "Failed to migrate ai_models schema (create ai_models_new): {err}"
            ))
        })?;

        sqlx::query(
            r#"
            INSERT INTO ai_models_new (
                id, provider, api_url, api_key_encrypted, model_name, display_name, model_type,
                config_json, created_at, updated_at,
                auth_type, oauth_provider, oauth_refresh_token_encrypted,
                oauth_access_token_encrypted, oauth_token_expires_at, oauth_metadata
            )
            SELECT
                id, provider, api_url, api_key_encrypted, model_name, model_name, model_type,
                config_json, created_at, updated_at,
                auth_type, oauth_provider, oauth_refresh_token_encrypted,
                oauth_access_token_encrypted, oauth_token_expires_at, oauth_metadata
            FROM ai_models
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            DatabaseError::internal(format!(
                "Failed to migrate ai_models schema (copy data): {err}"
            ))
        })?;

        sqlx::query("DROP TABLE ai_models")
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to migrate ai_models schema (drop old table): {err}"
                ))
            })?;

        sqlx::query("ALTER TABLE ai_models_new RENAME TO ai_models")
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to migrate ai_models schema (rename table): {err}"
                ))
            })?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to re-enable foreign_keys during ai_models migration: {err}"
                ))
            })?;

        tx.commit().await.map_err(|err| {
            DatabaseError::internal(format!("Failed to commit transaction: {err}"))
        })?;

        Ok(())
    }

    async fn ensure_workspaces_schema(&self) -> DatabaseResult<()> {
        let rows = sqlx::query("PRAGMA table_info(workspaces)")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to inspect workspaces schema: {err}"))
            })?;

        let mut has_selected_run_action_id = false;
        for row in &rows {
            if pragma_text_column(row, "name", "workspaces table_info")? == "selected_run_action_id"
            {
                has_selected_run_action_id = true;
                break;
            }
        }

        if !has_selected_run_action_id {
            let mut tx = self.pool.begin().await.map_err(|err| {
                DatabaseError::internal(format!("Failed to begin transaction: {err}"))
            })?;

            sqlx::query("ALTER TABLE workspaces ADD COLUMN selected_run_action_id TEXT")
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to migrate workspaces schema (add selected_run_action_id): {err}"
                    ))
                })?;

            tx.commit().await.map_err(|err| {
                DatabaseError::internal(format!("Failed to commit transaction: {err}"))
            })?;
        }

        Ok(())
    }

    async fn ensure_threads_schema(&self) -> DatabaseResult<()> {
        let rows = sqlx::query("PRAGMA table_info(threads)")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to inspect threads schema: {err}"))
            })?;

        let mut has_display_name = false;
        let mut has_thread_type = false;
        for row in &rows {
            let col_name = pragma_text_column(row, "name", "threads table_info")?;
            if col_name == "display_name" {
                has_display_name = true;
            }
            if col_name == "thread_type" {
                has_thread_type = true;
            }
        }

        if !has_display_name {
            sqlx::query("ALTER TABLE threads ADD COLUMN display_name TEXT")
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to migrate threads schema (add display_name): {err}"
                    ))
                })?;
        }

        if !has_thread_type {
            sqlx::query("ALTER TABLE threads ADD COLUMN thread_type TEXT NOT NULL DEFAULT 'agent'")
                .execute(&self.pool)
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!(
                        "Failed to migrate threads schema (add thread_type): {err}"
                    ))
                })?;
        }

        Ok(())
    }

    async fn insert_default_data(&self) -> DatabaseResult<()> {
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
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to insert default AI config `{feature_name}`: {err}"
                ))
            })?;
        }

        Ok(())
    }
}

struct KeyVault {
    path: PathBuf,
    key: OnceLock<[u8; 32]>,
}

impl KeyVault {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            key: OnceLock::new(),
        }
    }

    async fn master_key(&self) -> DatabaseResult<[u8; 32]> {
        if let Some(&key) = self.key.get() {
            return Ok(key);
        }

        let key = match self.load_from_disk().await? {
            Some(key) => key,
            None => self.derive_from_device().await?,
        };

        if self.key.set(key).is_err() {
            tracing::warn!("master key cache was already initialized before set");
        }
        Ok(key)
    }

    async fn load_from_disk(&self) -> DatabaseResult<Option<[u8; 32]>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = tokio::fs::read_to_string(&self.path).await.map_err(|err| {
            DatabaseError::io(format!("read key file {}", self.path.display()), err)
        })?;
        let mut lines = raw.lines();
        let Some(first) = lines.next() else {
            return Ok(None);
        };
        let encoded = if first == KEY_FILE_VERSION {
            lines.next().ok_or_else(|| {
                DatabaseError::internal(format!(
                    "Invalid key file format at {}: missing payload after version header",
                    self.path.display()
                ))
            })?
        } else {
            first
        };
        if encoded.is_empty() {
            return Ok(None);
        }
        let decoded = BASE64.decode(encoded).map_err(DatabaseError::from)?;
        if decoded.len() != 32 {
            return Err(DatabaseError::InvalidKeyLength);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Some(bytes))
    }

    async fn derive_from_device(&self) -> DatabaseResult<[u8; 32]> {
        let device_id = self.get_device_identifier()?;

        let mut hasher = Sha256::new();
        hasher.update(device_id.as_bytes());
        hasher.update(b"orbitx-secret-v1");

        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);

        self.persist(bytes).await?;
        Ok(bytes)
    }

    fn get_device_identifier(&self) -> DatabaseResult<String> {
        // Use machine-uid crate to get cross-platform machine unique identifier
        // Windows: Read from registry HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography\MachineGuid
        // macOS: Use ioreg to get IOPlatformUUID
        // Linux: Read /var/lib/dbus/machine-id or /etc/machine-id
        match machine_uid::get() {
            Ok(uid) => Ok(uid),
            Err(err) => Err(DatabaseError::internal(format!(
                "Failed to get machine UID: {err}"
            ))),
        }
    }

    async fn persist(&self, bytes: [u8; 32]) -> DatabaseResult<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                DatabaseError::io(format!("create key directory {}", parent.display()), err)
            })?;
        }
        let encoded = BASE64.encode(bytes);
        let payload = format!("{KEY_FILE_VERSION}\n{encoded}\n");
        let tmp_path = self.path.with_extension("tmp");
        tokio::fs::write(&tmp_path, payload.as_bytes())
            .await
            .map_err(|err| {
                DatabaseError::io(format!("write key temp file {}", tmp_path.display()), err)
            })?;
        tokio::fs::rename(&tmp_path, &self.path)
            .await
            .map_err(|err| {
                DatabaseError::io(format!("replace key file {}", self.path.display()), err)
            })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&self.path)
                .await
                .map_err(|err| DatabaseError::io("read key file metadata", err))?
                .permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&self.path, perms)
                .await
                .map_err(|err| DatabaseError::io("set key file permissions", err))?;
        }

        Ok(())
    }
}

fn pragma_text_column(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
    context: &str,
) -> DatabaseResult<String> {
    row.try_get(column).map_err(|err| {
        DatabaseError::internal(format!("Failed to read `{column}` from {context}: {err}"))
    })
}

fn pragma_i64_column(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
    context: &str,
) -> DatabaseResult<i64> {
    row.try_get(column).map_err(|err| {
        DatabaseError::internal(format!("Failed to read `{column}` from {context}: {err}"))
    })
}

fn resolve_sql_dir(options: &DatabaseOptions) -> DatabaseResult<PathBuf> {
    if let Some(custom) = &options.sql_dir {
        return Ok(custom.clone());
    }

    if cfg!(debug_assertions) {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql"))
    } else {
        let exe = std::env::current_exe().map_err(|err| {
            DatabaseError::internal(format!("Failed to get current executable path: {err}"))
        })?;

        if let Some(contents) = exe
            .ancestors()
            .find(|p| p.file_name() == Some(std::ffi::OsStr::new("Contents")))
        {
            Ok(contents.join("Resources/sql"))
        } else {
            let parent = exe.parent().ok_or_else(|| {
                DatabaseError::internal(format!(
                    "Executable path '{}' has no parent directory",
                    exe.display()
                ))
            })?;
            Ok(parent.join("sql"))
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
