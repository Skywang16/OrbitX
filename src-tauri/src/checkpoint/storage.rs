//! Checkpoint 数据访问层

use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;

use super::models::{
    Checkpoint, CheckpointResult, CheckpointSummary, FileSnapshot, NewCheckpoint, NewFileSnapshot,
};

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Checkpoint 数据访问
pub struct CheckpointStorage {
    pool: SqlitePool,
}

impl CheckpointStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, checkpoint: &NewCheckpoint) -> CheckpointResult<i64> {
        let now = now_timestamp();
        let result = sqlx::query(
            "INSERT INTO checkpoints (workspace_path, session_id, message_id, parent_id, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&checkpoint.workspace_path)
        .bind(checkpoint.session_id)
        .bind(checkpoint.message_id)
        .bind(checkpoint.parent_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> CheckpointResult<Option<Checkpoint>> {
        let row = sqlx::query(
            "SELECT id, workspace_path, session_id, message_id, parent_id, created_at
             FROM checkpoints WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| Checkpoint::from_row(&r)).transpose()
    }

    pub async fn find_by_message_id(
        &self,
        message_id: i64,
    ) -> CheckpointResult<Option<Checkpoint>> {
        let row = sqlx::query(
            "SELECT id, workspace_path, session_id, message_id, parent_id, created_at
             FROM checkpoints WHERE message_id = ?",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| Checkpoint::from_row(&r)).transpose()
    }

    pub async fn find_latest_by_session(
        &self,
        session_id: i64,
        workspace_path: &str,
    ) -> CheckpointResult<Option<Checkpoint>> {
        let row = sqlx::query(
            "SELECT id, workspace_path, session_id, message_id, parent_id, created_at
             FROM checkpoints
             WHERE session_id = ? AND workspace_path = ?
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(session_id)
        .bind(workspace_path)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| Checkpoint::from_row(&r)).transpose()
    }

    pub async fn list_summaries_by_session(
        &self,
        session_id: i64,
        workspace_path: &str,
    ) -> CheckpointResult<Vec<CheckpointSummary>> {
        let rows = sqlx::query(
            "SELECT
                c.id, c.workspace_path, c.session_id, c.message_id, c.parent_id, c.created_at,
                COUNT(f.id) as file_count,
                COALESCE(SUM(f.file_size), 0) as total_size
             FROM checkpoints c
             LEFT JOIN checkpoint_file_snapshots f ON c.id = f.checkpoint_id
             WHERE c.session_id = ? AND c.workspace_path = ?
             GROUP BY c.id
             ORDER BY c.created_at DESC",
        )
        .bind(session_id)
        .bind(workspace_path)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(CheckpointSummary::from_row)
            .collect()
    }

    pub async fn delete(&self, id: i64) -> CheckpointResult<()> {
        sqlx::query("DELETE FROM checkpoints WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // === FileSnapshot 操作 ===

    pub async fn insert_file_snapshots(
        &self,
        snapshots: &[NewFileSnapshot],
    ) -> CheckpointResult<()> {
        let now = now_timestamp();
        for snapshot in snapshots {
            sqlx::query(
                "INSERT INTO checkpoint_file_snapshots
                 (checkpoint_id, relative_path, blob_hash, change_type, file_size, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
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

    pub async fn insert_file_snapshot(&self, snapshot: &NewFileSnapshot) -> CheckpointResult<()> {
        self.insert_file_snapshots(std::slice::from_ref(snapshot))
            .await
    }

    pub async fn has_file_snapshot(
        &self,
        checkpoint_id: i64,
        relative_path: &str,
    ) -> CheckpointResult<bool> {
        let row = sqlx::query(
            "SELECT 1 FROM checkpoint_file_snapshots
             WHERE checkpoint_id = ? AND relative_path = ? LIMIT 1",
        )
        .bind(checkpoint_id)
        .bind(relative_path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    pub async fn find_file_snapshots(
        &self,
        checkpoint_id: i64,
    ) -> CheckpointResult<Vec<FileSnapshot>> {
        let rows = sqlx::query(
            "SELECT id, checkpoint_id, relative_path AS file_path, blob_hash, change_type, file_size, created_at
             FROM checkpoint_file_snapshots
             WHERE checkpoint_id = ?
             ORDER BY relative_path",
        )
        .bind(checkpoint_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(FileSnapshot::from_row).collect()
    }

    pub async fn find_file_snapshot(
        &self,
        checkpoint_id: i64,
        file_path: &str,
    ) -> CheckpointResult<Option<FileSnapshot>> {
        let row = sqlx::query(
            "SELECT id, checkpoint_id, relative_path AS file_path, blob_hash, change_type, file_size, created_at
             FROM checkpoint_file_snapshots
             WHERE checkpoint_id = ? AND relative_path = ?",
        )
        .bind(checkpoint_id)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| FileSnapshot::from_row(&r)).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_path TEXT NOT NULL,
                session_id INTEGER NOT NULL,
                message_id INTEGER NOT NULL,
                parent_id INTEGER,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE checkpoint_file_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                blob_hash TEXT NOT NULL,
                change_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE (checkpoint_id, relative_path)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_insert_and_find() {
        let pool = setup_test_db().await;
        let storage = CheckpointStorage::new(pool);

        let checkpoint = NewCheckpoint {
            workspace_path: "/tmp/project".to_string(),
            session_id: 1,
            message_id: 100,
            parent_id: None,
        };

        let id = storage.insert(&checkpoint).await.unwrap();
        let found = storage.find_by_id(id).await.unwrap().unwrap();

        assert_eq!(found.id, id);
        assert_eq!(found.session_id, 1);
        assert_eq!(found.message_id, 100);
        assert_eq!(found.workspace_path, "/tmp/project");
    }

    #[tokio::test]
    async fn test_find_by_message_id() {
        let pool = setup_test_db().await;
        let storage = CheckpointStorage::new(pool);

        let checkpoint = NewCheckpoint {
            workspace_path: "/tmp/project".to_string(),
            session_id: 1,
            message_id: 200,
            parent_id: None,
        };

        storage.insert(&checkpoint).await.unwrap();
        let found = storage.find_by_message_id(200).await.unwrap().unwrap();

        assert_eq!(found.message_id, 200);
    }
}
