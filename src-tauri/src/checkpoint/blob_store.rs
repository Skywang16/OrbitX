//! BlobStore：内容寻址存储
//!
//! 使用 SHA-256 哈希作为内容标识符，实现去重存储
//! 支持流式处理大文件

use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::config::CheckpointConfig;
use super::models::CheckpointResult;

/// 内容寻址存储
pub struct BlobStore {
    pool: SqlitePool,
    config: CheckpointConfig,
}

impl BlobStore {
    pub fn new(pool: SqlitePool, config: CheckpointConfig) -> Self {
        Self { pool, config }
    }

    /// 计算内容的 SHA-256 哈希
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 存储内容，返回 SHA-256 哈希
    /// 如果内容已存在，增加引用计数
    pub async fn store(&self, content: &[u8]) -> CheckpointResult<String> {
        // 检查文件大小限制
        if self.config.is_file_too_large(content.len() as u64) {
            return Err(super::models::CheckpointError::FileTooLarge(
                content.len() as u64
            ));
        }

        let hash = Self::compute_hash(content);

        // 检查是否已存在
        if self.exists(&hash).await? {
            self.increment_ref(&hash).await?;
            return Ok(hash);
        }

        let size = content.len() as i64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 插入新的 blob
        let result = sqlx::query(
            r#"
            INSERT INTO checkpoint_blobs (hash, content, size, ref_count, created_at)
            VALUES (?, ?, ?, 1, ?)
            "#,
        )
        .bind(&hash)
        .bind(content)
        .bind(size)
        .bind(now)
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            "BlobStore: stored blob hash={}, size={}, rows_affected={}",
            hash,
            size,
            result.rows_affected()
        );

        Ok(hash)
    }

    /// 流式存储大文件
    pub async fn store_stream<R: AsyncRead + Unpin>(
        &self,
        mut reader: R,
    ) -> CheckpointResult<String> {
        let mut hasher = Sha256::new();
        let mut content = Vec::new();
        let mut buffer = vec![0u8; self.config.stream_buffer_size];

        // 流式读取并计算哈希
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];
            hasher.update(chunk);
            content.extend_from_slice(chunk);

            // 检查文件大小限制
            if self.config.is_file_too_large(content.len() as u64) {
                return Err(super::models::CheckpointError::FileTooLarge(
                    content.len() as u64
                ));
            }
        }

        let hash = hex::encode(hasher.finalize());

        // 检查是否已存在
        if self.exists(&hash).await? {
            self.increment_ref(&hash).await?;
            return Ok(hash);
        }

        // 存储内容
        self.store_content(&hash, &content).await?;
        Ok(hash)
    }

    /// 存储内容的内部方法
    async fn store_content(&self, hash: &str, content: &[u8]) -> CheckpointResult<()> {
        let size = content.len() as i64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO checkpoint_blobs (hash, content, size, ref_count, created_at)
            VALUES (?, ?, ?, 1, ?)
            "#,
        )
        .bind(hash)
        .bind(content)
        .bind(size)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 根据哈希获取内容
    pub async fn get(&self, hash: &str) -> CheckpointResult<Option<Vec<u8>>> {
        let row = sqlx::query("SELECT content FROM checkpoint_blobs WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("content")))
    }

    /// 检查哈希是否存在
    pub async fn exists(&self, hash: &str) -> CheckpointResult<bool> {
        let row = sqlx::query("SELECT 1 FROM checkpoint_blobs WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.is_some())
    }

    /// 减少引用计数
    pub async fn decrement_ref(&self, hash: &str) -> CheckpointResult<()> {
        sqlx::query(
            r#"
            UPDATE checkpoint_blobs 
            SET ref_count = ref_count - 1 
            WHERE hash = ? AND ref_count > 0
            "#,
        )
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 增加引用计数
    pub async fn increment_ref(&self, hash: &str) -> CheckpointResult<()> {
        sqlx::query("UPDATE checkpoint_blobs SET ref_count = ref_count + 1 WHERE hash = ?")
            .bind(hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 垃圾回收：清理引用计数为 0 的 blob
    pub async fn gc(&self) -> CheckpointResult<u64> {
        let result = sqlx::query("DELETE FROM checkpoint_blobs WHERE ref_count <= 0")
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            tracing::info!("BlobStore GC: deleted {} orphaned blobs", deleted);
        }

        Ok(deleted)
    }

    /// 获取存储统计信息
    pub async fn get_stats(&self) -> CheckpointResult<BlobStoreStats> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as blob_count,
                SUM(size) as total_size,
                SUM(ref_count) as total_refs,
                COUNT(CASE WHEN ref_count = 0 THEN 1 END) as orphaned_count
            FROM checkpoint_blobs
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BlobStoreStats {
            blob_count: row.get("blob_count"),
            total_size: row.get("total_size"),
            total_refs: row.get("total_refs"),
            orphaned_count: row.get("orphaned_count"),
        })
    }
}

/// BlobStore 统计信息
#[derive(Debug, Clone)]
pub struct BlobStoreStats {
    pub blob_count: i64,
    pub total_size: i64,
    pub total_refs: i64,
    pub orphaned_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE checkpoint_blobs (
                hash TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                size INTEGER NOT NULL,
                ref_count INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let pool = setup_test_db().await;
        let config = CheckpointConfig::default();
        let store = BlobStore::new(pool, config);

        let content = b"Hello, World!";
        let hash = store.store(content).await.unwrap();

        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(content, retrieved.as_slice());
    }

    #[tokio::test]
    async fn test_deduplication() {
        let pool = setup_test_db().await;
        let config = CheckpointConfig::default();
        let store = BlobStore::new(pool, config);

        let content = b"Hello, World!";
        let hash1 = store.store(content).await.unwrap();
        let hash2 = store.store(content).await.unwrap();

        assert_eq!(hash1, hash2);

        let stats = store.get_stats().await.unwrap();
        assert_eq!(stats.blob_count, 1);
        assert_eq!(stats.total_refs, 2);
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let pool = setup_test_db().await;
        let mut config = CheckpointConfig::default();
        config.max_file_size = 10; // 10 bytes limit
        let store = BlobStore::new(pool, config);

        let large_content = vec![0u8; 20]; // 20 bytes
        let result = store.store(&large_content).await;

        assert!(matches!(
            result,
            Err(super::models::CheckpointError::FileTooLarge(_))
        ));
    }
}
