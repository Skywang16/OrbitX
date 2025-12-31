//! CheckpointStorage：数据访问层
//!
//! 负责 checkpoint 和文件快照的数据库 CRUD 操作

use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use super::models::{
    Checkpoint, CheckpointResult, CheckpointSummary, FileChangeType, FileSnapshot, NewCheckpoint,
    NewFileSnapshot,
};

/// Checkpoint 数据访问层
pub struct CheckpointStorage {
    pool: SqlitePool,
}

impl CheckpointStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取当前时间戳
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    // ==================== Checkpoint CRUD ====================

    /// 插入新的 checkpoint
    pub async fn insert_checkpoint(&self, checkpoint: &NewCheckpoint) -> CheckpointResult<i64> {
        let now = Self::now();

        let result = sqlx::query(
            r#"
            INSERT INTO checkpoints (workspace_path, session_id, parent_id, user_message, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&checkpoint.workspace_path)
        .bind(checkpoint.session_id)
        .bind(checkpoint.parent_id)
        .bind(&checkpoint.user_message)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        tracing::debug!(
            "CheckpointStorage: inserted checkpoint id={}, workspace_path={}, session_id={}",
            id,
            checkpoint.workspace_path,
            checkpoint.session_id
        );

        Ok(id)
    }

    /// 根据 ID 获取 checkpoint
    pub async fn get_checkpoint(&self, id: i64) -> CheckpointResult<Option<Checkpoint>> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_path, session_id, parent_id, user_message, created_at
            FROM checkpoints
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            Checkpoint::new(
                r.get("id"),
                r.get("workspace_path"),
                r.get("session_id"),
                r.get("parent_id"),
                r.get("user_message"),
                r.get("created_at"),
            )
        }))
    }

    /// 获取工作区的所有 checkpoint（按时间倒序）
    pub async fn list_by_workspace(
        &self,
        workspace_path: &str,
    ) -> CheckpointResult<Vec<Checkpoint>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_path, session_id, parent_id, user_message, created_at
            FROM checkpoints
            WHERE workspace_path = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(workspace_path)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                Checkpoint::new(
                    r.get("id"),
                    r.get("workspace_path"),
                    r.get("session_id"),
                    r.get("parent_id"),
                    r.get("user_message"),
                    r.get("created_at"),
                )
            })
            .collect())
    }

    /// 获取工作区的 checkpoint 摘要列表（包含统计信息）
    pub async fn list_summaries_by_workspace(
        &self,
        workspace_path: &str,
    ) -> CheckpointResult<Vec<CheckpointSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                c.id, c.workspace_path, c.session_id, c.parent_id, c.user_message, c.created_at,
                COUNT(f.id) as file_count,
                COALESCE(SUM(f.file_size), 0) as total_size
            FROM checkpoints c
            LEFT JOIN checkpoint_file_snapshots f ON c.id = f.checkpoint_id
            WHERE c.workspace_path = ?
            GROUP BY c.id
            ORDER BY c.created_at DESC
            "#,
        )
        .bind(workspace_path)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| CheckpointSummary {
                id: r.get("id"),
                workspace_path: r.get("workspace_path"),
                session_id: r.get("session_id"),
                parent_id: r.get("parent_id"),
                user_message: r.get("user_message"),
                created_at: super::models::timestamp_to_datetime(r.get("created_at")),
                file_count: r.get("file_count"),
                total_size: r.get("total_size"),
            })
            .collect())
    }

    /// 获取会话的 checkpoint 摘要列表
    pub async fn list_summaries_by_session(
        &self,
        session_id: i64,
    ) -> CheckpointResult<Vec<CheckpointSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                c.id, c.workspace_path, c.session_id, c.parent_id, c.user_message, c.created_at,
                COUNT(f.id) as file_count,
                COALESCE(SUM(f.file_size), 0) as total_size
            FROM checkpoints c
            LEFT JOIN checkpoint_file_snapshots f ON c.id = f.checkpoint_id
            WHERE c.session_id = ?
            GROUP BY c.id
            ORDER BY c.created_at DESC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| CheckpointSummary {
                id: r.get("id"),
                workspace_path: r.get("workspace_path"),
                session_id: r.get("session_id"),
                parent_id: r.get("parent_id"),
                user_message: r.get("user_message"),
                created_at: super::models::timestamp_to_datetime(r.get("created_at")),
                file_count: r.get("file_count"),
                total_size: r.get("total_size"),
            })
            .collect())
    }

    /// 获取会话的最新 checkpoint
    pub async fn get_latest_checkpoint_for_session(
        &self,
        session_id: i64,
    ) -> CheckpointResult<Option<Checkpoint>> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_path, session_id, parent_id, user_message, created_at
            FROM checkpoints
            WHERE session_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            Checkpoint::new(
                r.get("id"),
                r.get("workspace_path"),
                r.get("session_id"),
                r.get("parent_id"),
                r.get("user_message"),
                r.get("created_at"),
            )
        }))
    }

    /// 删除 checkpoint
    pub async fn delete_checkpoint(&self, id: i64) -> CheckpointResult<()> {
        sqlx::query("DELETE FROM checkpoints WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        tracing::debug!("CheckpointStorage: deleted checkpoint id={}", id);
        Ok(())
    }

    // ==================== FileSnapshot CRUD ====================

    /// 插入文件快照
    pub async fn insert_file_snapshot(&self, snapshot: &NewFileSnapshot) -> CheckpointResult<i64> {
        let now = Self::now();

        let result = sqlx::query(
            r#"
            INSERT INTO checkpoint_file_snapshots 
                (checkpoint_id, file_path, blob_hash, change_type, file_size, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(snapshot.checkpoint_id)
        .bind(&snapshot.file_path)
        .bind(&snapshot.blob_hash)
        .bind(snapshot.change_type.as_str())
        .bind(snapshot.file_size)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 批量插入文件快照
    pub async fn insert_file_snapshots(
        &self,
        snapshots: &[NewFileSnapshot],
    ) -> CheckpointResult<()> {
        let now = Self::now();

        for snapshot in snapshots {
            sqlx::query(
                r#"
                INSERT INTO checkpoint_file_snapshots 
                    (checkpoint_id, file_path, blob_hash, change_type, file_size, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(snapshot.checkpoint_id)
            .bind(&snapshot.file_path)
            .bind(&snapshot.blob_hash)
            .bind(snapshot.change_type.as_str())
            .bind(snapshot.file_size)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// 获取 checkpoint 的所有文件快照
    pub async fn get_file_snapshots(
        &self,
        checkpoint_id: i64,
    ) -> CheckpointResult<Vec<FileSnapshot>> {
        let rows = sqlx::query(
            r#"
            SELECT id, checkpoint_id, file_path, blob_hash, change_type, file_size, created_at
            FROM checkpoint_file_snapshots
            WHERE checkpoint_id = ?
            ORDER BY file_path
            "#,
        )
        .bind(checkpoint_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(FileSnapshot::new(
                    r.get("id"),
                    r.get("checkpoint_id"),
                    r.get("file_path"),
                    r.get("blob_hash"),
                    FileChangeType::from_str(&r.get::<String, _>("change_type"))?,
                    r.get("file_size"),
                    r.get("created_at"),
                ))
            })
            .collect()
    }

    /// 获取指定文件在 checkpoint 中的快照
    pub async fn get_file_snapshot(
        &self,
        checkpoint_id: i64,
        file_path: &str,
    ) -> CheckpointResult<Option<FileSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT id, checkpoint_id, file_path, blob_hash, change_type, file_size, created_at
            FROM checkpoint_file_snapshots
            WHERE checkpoint_id = ? AND file_path = ?
            "#,
        )
        .bind(checkpoint_id)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            Ok(FileSnapshot::new(
                r.get("id"),
                r.get("checkpoint_id"),
                r.get("file_path"),
                r.get("blob_hash"),
                FileChangeType::from_str(&r.get::<String, _>("change_type"))?,
                r.get("file_size"),
                r.get("created_at"),
            ))
        })
        .transpose()
    }

    /// 获取 checkpoint 引用的所有 blob hash
    pub async fn get_blob_hashes(&self, checkpoint_id: i64) -> CheckpointResult<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT blob_hash FROM checkpoint_file_snapshots WHERE checkpoint_id = ?",
        )
        .bind(checkpoint_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.get("blob_hash")).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_path TEXT NOT NULL,
                session_id INTEGER NOT NULL,
                parent_id INTEGER,
                user_message TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE checkpoint_file_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                blob_hash TEXT NOT NULL,
                change_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE (checkpoint_id, file_path)
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_insert_and_get_checkpoint() {
        let pool = setup_test_db().await;
        let storage = CheckpointStorage::new(pool);

        let new_cp = NewCheckpoint {
            workspace_path: "/tmp/project".to_string(),
            session_id: 1,
            parent_id: None,
            user_message: "test message".to_string(),
        };

        let id = storage.insert_checkpoint(&new_cp).await.unwrap();
        let cp = storage.get_checkpoint(id).await.unwrap().unwrap();

        assert_eq!(cp.id, id);
        assert_eq!(cp.session_id, 1);
        assert_eq!(cp.workspace_path, "/tmp/project");
        assert_eq!(cp.user_message, "test message");
        assert!(cp.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_list_by_workspace() {
        let pool = setup_test_db().await;
        let storage = CheckpointStorage::new(pool);

        for i in 0..3 {
            let new_cp = NewCheckpoint {
                workspace_path: "/tmp/project".to_string(),
                session_id: 1,
                parent_id: None,
                user_message: format!("message {}", i),
            };
            storage.insert_checkpoint(&new_cp).await.unwrap();
        }

        let list = storage
            .list_by_workspace("/tmp/project")
            .await
            .unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_file_snapshots() {
        let pool = setup_test_db().await;
        let storage = CheckpointStorage::new(pool);

        let new_cp = NewCheckpoint {
            workspace_path: "/tmp/project".to_string(),
            session_id: 1,
            parent_id: None,
            user_message: "test".to_string(),
        };
        let cp_id = storage.insert_checkpoint(&new_cp).await.unwrap();

        let snapshot = NewFileSnapshot {
            checkpoint_id: cp_id,
            file_path: "/test/file.rs".to_string(),
            blob_hash: "abc123".to_string(),
            change_type: FileChangeType::Added,
            file_size: 100,
        };
        storage.insert_file_snapshot(&snapshot).await.unwrap();

        let snapshots = storage.get_file_snapshots(cp_id).await.unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].file_path, "/test/file.rs");
        assert_eq!(snapshots[0].change_type, FileChangeType::Added);
    }
}
