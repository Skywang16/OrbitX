use crate::storage::paths::StoragePaths;
use crate::storage::types::SessionState;
use crate::storage::SESSION_STATE_FILE_NAME;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use chrono::Utc;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

const MAGIC: &[u8; 4] = b"OXMP";
const VERSION: u8 = 1;
const FLAG_COMPRESSED: u8 = 0b0000_0001;
const FLAG_CHECKSUM: u8 = 0b0000_0010;
const HEADER_LEN: usize = 4 + 1 + 1 + 8 + 32;

#[derive(Debug, Clone)]
pub struct MessagePackOptions {
    pub compression: bool,
    pub backup_count: usize,
    pub checksum_validation: bool,
    pub max_file_size: usize,
}

impl Default for MessagePackOptions {
    fn default() -> Self {
        Self {
            compression: true,
            backup_count: 3,
            checksum_validation: true,
            max_file_size: 10 * 1024 * 1024,
        }
    }
}

pub struct MessagePackManager {
    paths: StoragePaths,
    options: MessagePackOptions,
}

impl MessagePackManager {
    pub async fn new(paths: StoragePaths, options: MessagePackOptions) -> AppResult<Self> {
        let manager = Self { paths, options };
        manager.ensure_state_directory().await?;
        Ok(manager)
    }

    pub fn serialize_state(&self, state: &SessionState) -> AppResult<Vec<u8>> {
        let (payload, flags) = if self.options.compression {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            state
                .serialize(&mut Serializer::new(&mut encoder))
                .map_err(|e| anyhow!("MessagePack序列化失败: {}", e))?;
            let compressed = encoder.finish().map_err(|e| anyhow!("压缩失败: {}", e))?;
            (compressed, FLAG_COMPRESSED)
        } else {
            let mut buf = Vec::new();
            state
                .serialize(&mut Serializer::new(&mut buf))
                .map_err(|e| anyhow!("MessagePack序列化失败: {}", e))?;
            (buf, 0)
        };

        if payload.len() > self.options.max_file_size {
            return Err(anyhow!("序列化数据过大: {} bytes", payload.len()));
        }

        let mut header = [0u8; HEADER_LEN];
        header[..4].copy_from_slice(MAGIC);
        header[4] = VERSION;
        header[5] = flags
            | if self.options.checksum_validation {
                FLAG_CHECKSUM
            } else {
                0
            };
        header[6..14].copy_from_slice(&(payload.len() as u64).to_le_bytes());

        if self.options.checksum_validation {
            let mut hasher = Sha256::new();
            hasher.update(&payload);
            let digest = hasher.finalize();
            header[14..46].copy_from_slice(&digest[..32]);
        }

        let mut result = Vec::with_capacity(HEADER_LEN + payload.len());
        result.extend_from_slice(&header);
        result.extend_from_slice(&payload);
        Ok(result)
    }

    pub fn deserialize_state(&self, data: &[u8]) -> AppResult<SessionState> {
        if data.len() < HEADER_LEN {
            return Err(anyhow!("状态文件头无效"));
        }

        let header = &data[..HEADER_LEN];
        if &header[..4] != MAGIC {
            return Err(anyhow!("状态文件魔数错误"));
        }
        if header[4] != VERSION {
            return Err(anyhow!("状态文件版本不支持"));
        }

        let flags = header[5];
        let payload_len = u64::from_le_bytes(header[6..14].try_into().unwrap()) as usize;
        if HEADER_LEN + payload_len != data.len() {
            return Err(anyhow!("状态文件长度不一致"));
        }

        let payload = &data[HEADER_LEN..];

        if (flags & FLAG_CHECKSUM) != 0 {
            let mut hasher = Sha256::new();
            hasher.update(payload);
            let digest = hasher.finalize();
            if digest.as_slice() != &header[14..46] {
                return Err(anyhow!("状态文件校验失败"));
            }
        }

        let decoded = if (flags & FLAG_COMPRESSED) != 0 {
            let mut decoder = GzDecoder::new(payload);
            let mut buf = Vec::new();
            decoder
                .read_to_end(&mut buf)
                .map_err(|e| anyhow!("状态解压失败: {}", e))?;
            buf
        } else {
            payload.to_vec()
        };

        let mut deserializer = Deserializer::new(&decoded[..]);
        SessionState::deserialize(&mut deserializer).map_err(|e| anyhow!("状态反序列化失败: {}", e))
    }

    pub async fn save_state(&self, state: &SessionState) -> AppResult<()> {
        self.ensure_state_directory().await?;
        let serialized = self.serialize_state(state)?;
        let state_file = self.get_state_file_path();

        if state_file.exists() {
            self.create_backup(&state_file).await?;
        }

        self.atomic_write(&state_file, &serialized).await?;
        self.cleanup_old_backups().await?;
        Ok(())
    }

    pub async fn load_state(&self) -> AppResult<Option<SessionState>> {
        let state_file = self.get_state_file_path();
        if !state_file.exists() {
            return Ok(None);
        }

        let data = async_fs::read(&state_file)
            .await
            .with_context(|| format!("读取状态文件失败: {}", state_file.display()))?;

        match self.deserialize_state(&data) {
            Ok(state) => Ok(Some(state)),
            Err(err) => {
                tracing::warn!("状态文件损坏: {}", err);
                self.restore_from_backup().await
            }
        }
    }

    pub async fn create_backup(&self, source_file: &Path) -> AppResult<PathBuf> {
        let backup_dir = self.get_backup_directory();
        async_fs::create_dir_all(&backup_dir)
            .await
            .with_context(|| format!("创建备份目录失败: {}", backup_dir.display()))?;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("session_state_{}.msgpack.bak", timestamp));
        async_fs::copy(source_file, &backup_file)
            .await
            .with_context(|| format!("创建备份失败: {}", backup_file.display()))?;
        Ok(backup_file)
    }

    pub async fn restore_from_backup(&self) -> AppResult<Option<SessionState>> {
        let backup_dir = self.get_backup_directory();
        if !backup_dir.exists() {
            return Ok(None);
        }

        let mut backups = Vec::new();
        let mut entries = async_fs::read_dir(&backup_dir)
            .await
            .with_context(|| format!("读取备份目录失败: {}", backup_dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("遍历备份目录失败: {}", backup_dir.display()))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bak") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        backups.push((path, modified));
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.1.cmp(&a.1));

        for (backup, _) in backups {
            let data = match async_fs::read(&backup).await {
                Ok(data) => data,
                Err(_) => continue,
            };
            if let Ok(state) = self.deserialize_state(&data) {
                return Ok(Some(state));
            }
        }

        Ok(None)
    }

    async fn atomic_write(&self, target: &Path, data: &[u8]) -> AppResult<()> {
        let tmp = target.with_extension("tmp");
        async_fs::write(&tmp, data)
            .await
            .with_context(|| format!("写入临时文件失败: {}", tmp.display()))?;
        async_fs::rename(&tmp, target).await.map_err(|err| {
            let _ = std::fs::remove_file(&tmp);
            anyhow!(
                "原子重命名失败: {} -> {}: {}",
                tmp.display(),
                target.display(),
                err
            )
        })?;
        Ok(())
    }

    async fn cleanup_old_backups(&self) -> AppResult<()> {
        let backup_dir = self.get_backup_directory();
        if !backup_dir.exists() {
            return Ok(());
        }

        let mut backups = Vec::new();
        let mut entries = async_fs::read_dir(&backup_dir)
            .await
            .with_context(|| format!("读取备份目录失败: {}", backup_dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("遍历备份目录失败: {}", backup_dir.display()))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bak") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        backups.push((path, modified));
                    }
                }
            }
        }

        let excess = backups.len().saturating_sub(self.options.backup_count);
        if excess == 0 {
            return Ok(());
        }

        backups.sort_by(|a, b| a.1.cmp(&b.1));
        for (path, _) in backups.into_iter().take(excess) {
            let _ = async_fs::remove_file(path).await;
        }
        Ok(())
    }

    async fn ensure_state_directory(&self) -> AppResult<()> {
        async_fs::create_dir_all(&self.paths.state_dir)
            .await
            .with_context(|| format!("创建状态目录失败: {}", self.paths.state_dir.display()))
    }

    fn get_state_file_path(&self) -> PathBuf {
        self.paths.state_dir.join(SESSION_STATE_FILE_NAME)
    }

    fn get_backup_directory(&self) -> PathBuf {
        self.paths.backups_dir.join("state")
    }

    pub async fn get_state_stats(&self) -> AppResult<StateStats> {
        let state_file = self.get_state_file_path();
        let backup_dir = self.get_backup_directory();

        let state_size = if state_file.exists() {
            async_fs::metadata(&state_file)
                .await
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        let mut backup_count = 0;
        let mut backup_size = 0;

        if backup_dir.exists() {
            let mut entries = async_fs::read_dir(&backup_dir)
                .await
                .with_context(|| format!("读取备份目录失败: {}", backup_dir.display()))?;
            while let Some(entry) = entries
                .next_entry()
                .await
                .with_context(|| format!("遍历备份目录失败: {}", backup_dir.display()))?
            {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("bak") {
                    backup_count += 1;
                    if let Ok(metadata) = entry.metadata().await {
                        backup_size += metadata.len();
                    }
                }
            }
        }

        Ok(StateStats {
            state_file_exists: state_file.exists(),
            state_file_size: state_size,
            backup_count,
            backup_total_size: backup_size,
            compression_enabled: self.options.compression,
            checksum_validation_enabled: self.options.checksum_validation,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStats {
    pub state_file_exists: bool,
    pub state_file_size: u64,
    pub backup_count: usize,
    pub backup_total_size: u64,
    pub compression_enabled: bool,
    pub checksum_validation_enabled: bool,
}

impl StateStats {
    pub fn state_file_size_formatted(&self) -> String {
        format_bytes(self.state_file_size)
    }

    pub fn backup_total_size_formatted(&self) -> String {
        format_bytes(self.backup_total_size)
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.2} {}", size, UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mock_state() -> SessionState {
        SessionState::default()
    }

    #[test]
    fn serialize_header_structure() {
        let temp_dir = TempDir::new().unwrap();
        let paths = StoragePaths::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = MessagePackManager {
            paths,
            options: MessagePackOptions::default(),
        };
        let data = manager.serialize_state(&mock_state()).unwrap();
        assert!(data.len() > HEADER_LEN);
        assert_eq!(&data[..4], MAGIC);
        assert_eq!(data[4], VERSION);
    }

    #[test]
    fn deserialize_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let paths = StoragePaths::new(temp_dir.path().to_path_buf()).unwrap();
        let manager = MessagePackManager {
            paths,
            options: MessagePackOptions::default(),
        };
        let state = mock_state();
        let encoded = manager.serialize_state(&state).unwrap();
        let decoded = manager.deserialize_state(&encoded).unwrap();
        assert_eq!(decoded.version, state.version);
    }

    #[tokio::test]
    async fn save_and_load_state() {
        let temp_dir = TempDir::new().unwrap();
        let paths = StoragePaths::new(temp_dir.path().to_path_buf()).unwrap();
        paths.ensure_directories().unwrap();
        let manager = MessagePackManager::new(paths, MessagePackOptions::default())
            .await
            .unwrap();
        let state = mock_state();
        manager.save_state(&state).await.unwrap();
        let loaded = manager.load_state().await.unwrap().unwrap();
        assert_eq!(loaded.version, state.version);
    }
}
