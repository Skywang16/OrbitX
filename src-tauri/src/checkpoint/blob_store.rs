//! BlobStore：内容寻址存储
//!
//! 使用 SHA-256 哈希作为内容标识符，实现去重存储

use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

use super::models::CheckpointResult;

/// 内容寻址存储
pub struct BlobStore {
    pool: SqlitePool,
}

impl BlobStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
        let hash = Self::compute_hash(content);
        let size = content.len() as i64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 尝试插入，如果已存在则增加引用计数
        let result = sqlx::query(
            r#"
            INSERT INTO checkpoint_blobs (hash, content, size, ref_count, created_at)
            VALUES (?, ?, ?, 1, ?)
            ON CONFLICT(hash) DO UPDATE SET ref_count = ref_count + 1
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

    /// 获取 blob 的引用计数
    pub async fn get_ref_count(&self, hash: &str) -> CheckpointResult<Option<i64>> {
        let row = sqlx::query("SELECT ref_count FROM checkpoint_blobs WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("ref_count")))
    }

    /// 获取存储统计信息
    pub async fn stats(&self) -> CheckpointResult<BlobStoreStats> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as blob_count,
                COALESCE(SUM(size), 0) as total_size,
                COALESCE(SUM(ref_count), 0) as total_refs
            FROM checkpoint_blobs
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BlobStoreStats {
            blob_count: row.get("blob_count"),
            total_size: row.get("total_size"),
            total_refs: row.get("total_refs"),
        })
    }
}

/// BlobStore 统计信息
#[derive(Debug, Clone)]
pub struct BlobStoreStats {
    pub blob_count: i64,
    pub total_size: i64,
    pub total_refs: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的内存数据库
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE checkpoint_blobs (
                hash TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                size INTEGER NOT NULL,
                ref_count INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_compute_hash_deterministic() {
        let content = b"hello world";
        let hash1 = BlobStore::compute_hash(content);
        let hash2 = BlobStore::compute_hash(content);
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let pool = setup_test_db().await;
        let store = BlobStore::new(pool);

        let content = b"test content";
        let hash = store.store(content).await.unwrap();

        let retrieved = store.get(&hash).await.unwrap();
        assert_eq!(retrieved, Some(content.to_vec()));
    }

    #[tokio::test]
    async fn test_deduplication() {
        let pool = setup_test_db().await;
        let store = BlobStore::new(pool);

        let content = b"duplicate content";

        // 存储两次相同内容
        let hash1 = store.store(content).await.unwrap();
        let hash2 = store.store(content).await.unwrap();

        // 哈希应该相同
        assert_eq!(hash1, hash2);

        // 引用计数应该是 2
        let ref_count = store.get_ref_count(&hash1).await.unwrap();
        assert_eq!(ref_count, Some(2));

        // 统计应该只有 1 个 blob
        let stats = store.stats().await.unwrap();
        assert_eq!(stats.blob_count, 1);
    }

    #[tokio::test]
    async fn test_gc() {
        let pool = setup_test_db().await;
        let store = BlobStore::new(pool);

        let content = b"gc test";
        let hash = store.store(content).await.unwrap();

        // 减少引用计数到 0
        store.decrement_ref(&hash).await.unwrap();

        // GC 应该删除这个 blob
        let deleted = store.gc().await.unwrap();
        assert_eq!(deleted, 1);

        // blob 应该不存在了
        assert!(!store.exists(&hash).await.unwrap());
    }
}
