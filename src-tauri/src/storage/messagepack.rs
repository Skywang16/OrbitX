use crate::storage::paths::StoragePaths;
use crate::storage::types::SessionState;
use crate::storage::SESSION_STATE_FILE_NAME;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};

use chrono::{DateTime, Utc};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tracing::{debug, error, warn};

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
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializedSessionState {
    version: u32,
    timestamp: DateTime<Utc>,
    checksum: String,
    compressed: bool,
    data: Vec<u8>,
}

impl SerializedSessionState {
    fn new(state: &SessionState, compressed: bool, data: Vec<u8>) -> AppResult<Self> {
        let checksum = Self::calculate_checksum(&data)?;

        Ok(Self {
            version: state.version,
            timestamp: Utc::now(),
            checksum,
            compressed,
            data,
        })
    }

    fn calculate_checksum(data: &[u8]) -> AppResult<String> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    fn verify_checksum(&self) -> AppResult<bool> {
        let calculated = Self::calculate_checksum(&self.data)?;
        Ok(calculated == self.checksum)
    }
}

/// 负责会话状态的序列化、压缩存储和恢复
pub struct MessagePackManager {
    paths: StoragePaths,
    options: MessagePackOptions,
}

impl MessagePackManager {
    pub async fn new(paths: StoragePaths, options: MessagePackOptions) -> AppResult<Self> {
        let manager = Self { paths, options };

        // 确保状态目录存在
        manager.ensure_state_directory().await?;

        debug!(
            "配置: compression={}, backup_count={}, checksum_validation={}",
            manager.options.compression,
            manager.options.backup_count,
            manager.options.checksum_validation
        );

        Ok(manager)
    }

    pub fn serialize_state(&self, state: &SessionState) -> AppResult<Vec<u8>> {
        debug!("开始序列化会话状态");

        // 首先序列化为MessagePack格式
        let mut buf = Vec::new();
        state
            .serialize(&mut Serializer::new(&mut buf))
            .map_err(|e| anyhow!("MessagePack序列化失败: {}", e))?;

        debug!("MessagePack序列化完成，原始大小: {} bytes", buf.len());

        let (final_data, compressed) = if self.options.compression {
            let compressed_data = self.compress_data(&buf)?;
            debug!(
                "数据压缩完成，压缩后大小: {} bytes，压缩率: {:.2}%",
                compressed_data.len(),
                (1.0 - compressed_data.len() as f64 / buf.len() as f64) * 100.0
            );
            (compressed_data, true)
        } else {
            (buf, false)
        };

        let serialized = SerializedSessionState::new(state, compressed, final_data)?;

        // 序列化包装器
        let mut result = Vec::new();
        serialized
            .serialize(&mut Serializer::new(&mut result))
            .map_err(|e| anyhow!("包装器序列化失败: {}", e))?;

        if result.len() > self.options.max_file_size {
            return Err(anyhow!(
                "序列化数据过大: {} bytes，超过限制 {} bytes",
                result.len(),
                self.options.max_file_size
            ));
        }

        Ok(result)
    }

    pub fn deserialize_state(&self, data: &[u8]) -> AppResult<SessionState> {
        debug!("开始反序列化会话状态，数据大小: {} bytes", data.len());

        // 反序列化包装器
        let mut de = Deserializer::new(data);
        let serialized: SerializedSessionState =
            Deserialize::deserialize(&mut de).map_err(|e| anyhow!("包装器反序列化失败: {}", e))?;

        debug!(
            "包装器反序列化完成，版本: {}, 时间戳: {}, 压缩: {}",
            serialized.version, serialized.timestamp, serialized.compressed
        );

        // 验证校验和
        if self.options.checksum_validation && !serialized.verify_checksum()? {
            return Err(anyhow!("数据校验和验证失败"));
        }

        // 解压缩数据（如果需要）
        let state_data = if serialized.compressed {
            debug!("开始解压缩数据");
            self.decompress_data(&serialized.data)?
        } else {
            serialized.data
        };

        // 反序列化会话状态
        let mut de = Deserializer::new(&state_data[..]);
        let state: SessionState = Deserialize::deserialize(&mut de)
            .map_err(|e| anyhow!("会话状态反序列化失败: {}", e))?;

        Ok(state)
    }

    fn compress_data(&self, data: &[u8]) -> AppResult<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| anyhow!("数据压缩失败: {}", e))?;
        encoder.finish().map_err(|e| anyhow!("压缩完成失败: {}", e))
    }

    fn decompress_data(&self, data: &[u8]) -> AppResult<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut result = Vec::new();
        decoder
            .read_to_end(&mut result)
            .map_err(|e| anyhow!("数据解压缩失败: {}", e))?;
        Ok(result)
    }

    pub async fn save_state(&self, state: &SessionState) -> AppResult<()> {
        // 确保状态目录存在
        self.ensure_state_directory().await?;

        // 序列化状态
        let serialized_data = self.serialize_state(state)?;

        let state_file = self.get_state_file_path();

        if state_file.exists() {
            self.create_backup(&state_file).await?;
        }

        // 原子写入
        self.atomic_write(&state_file, &serialized_data).await?;

        // 清理旧备份
        self.cleanup_old_backups().await?;

        // 会话状态保存完成（静默）
        Ok(())
    }

    pub async fn load_state(&self) -> AppResult<Option<SessionState>> {
        let state_file = self.get_state_file_path();

        if !state_file.exists() {
            debug!("状态文件不存在: {}", state_file.display());
            return Ok(None);
        }

        // 读取文件
        let data = async_fs::read(&state_file)
            .await
            .with_context(|| format!("读取状态文件失败: {}", state_file.display()))?;

        // 尝试反序列化
        match self.deserialize_state(&data) {
            Ok(state) => Ok(Some(state)),
            Err(e) => {
                error!("状态文件反序列化失败: {}", e);

                // 尝试从备份恢复
                warn!("尝试从备份恢复状态");
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

        debug!("备份创建成功: {}", backup_file.display());
        Ok(backup_file)
    }

    pub async fn restore_from_backup(&self) -> AppResult<Option<SessionState>> {
        let backup_dir = self.get_backup_directory();

        if !backup_dir.exists() {
            warn!("备份目录不存在");
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

        // 按修改时间降序排序（最新的在前）
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // 尝试从最新的备份开始恢复
        for (backup_path, _) in backups {
            match async_fs::read(&backup_path).await {
                Ok(data) => match self.deserialize_state(&data) {
                    Ok(state) => return Ok(Some(state)),
                    Err(e) => {
                        warn!(
                            "备份文件损坏，尝试下一个: {} - {}",
                            backup_path.display(),
                            e
                        );
                        continue;
                    }
                },
                Err(e) => {
                    warn!("读取备份文件失败: {} - {}", backup_path.display(), e);
                    continue;
                }
            }
        }

        error!("所有备份文件都无法恢复");
        Ok(None)
    }

    async fn atomic_write(&self, target_path: &Path, data: &[u8]) -> AppResult<()> {
        let temp_path = target_path.with_extension("tmp");

        // 写入临时文件
        async_fs::write(&temp_path, data)
            .await
            .with_context(|| format!("写入临时文件失败: {}", temp_path.display()))?;

        // 原子重命名
        async_fs::rename(&temp_path, target_path)
            .await
            .map_err(|e| {
                // 清理临时文件
                let _ = std::fs::remove_file(&temp_path);
                anyhow!(
                    "原子重命名失败: {} -> {}: {}",
                    temp_path.display(),
                    target_path.display(),
                    e
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

        if backups.len() > self.options.backup_count {
            // 按修改时间升序排序（最旧的在前）
            backups.sort_by(|a, b| a.1.cmp(&b.1));

            let to_remove = backups.len() - self.options.backup_count;
            for (old_backup, _) in backups.iter().take(to_remove) {
                match async_fs::remove_file(old_backup).await {
                    Ok(_) => debug!("删除旧备份: {}", old_backup.display()),
                    Err(e) => warn!("删除旧备份失败: {} - {}", old_backup.display(), e),
                }
            }
        }

        Ok(())
    }

    async fn ensure_state_directory(&self) -> AppResult<()> {
        let state_dir = &self.paths.state_dir;
        async_fs::create_dir_all(&state_dir)
            .await
            .with_context(|| format!("创建状态目录失败: {}", state_dir.display()))?;
        Ok(())
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
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
