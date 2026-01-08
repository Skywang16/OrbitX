/*!
 * Completion learning model persistence (SQLite)
 *
 * 目标：
 * - 离线学习（无网络）
 * - 小体积（通过裁剪策略保持在 ~10MB）
 * - 数据结构驱动（key / transition / entity 三张表）
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::RepositoryResult;

pub struct CompletionModelRepo<'a> {
    db: &'a DatabaseManager,
}

impl<'a> CompletionModelRepo<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.db.pool()
    }

    pub async fn upsert_command_key(
        &self,
        key: &str,
        root: &str,
        sub: Option<&str>,
        used_ts: u64,
        success: Option<bool>,
    ) -> RepositoryResult<i64> {
        let (success_inc, fail_inc) = match success {
            Some(true) => (1_i64, 0_i64),
            Some(false) => (0_i64, 1_i64),
            None => (0_i64, 0_i64),
        };

        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO completion_command_keys
                (key, root, sub, use_count, success_count, fail_count, last_used_ts)
            VALUES
                (?, ?, ?, 1, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                use_count = use_count + 1,
                success_count = success_count + excluded.success_count,
                fail_count = fail_count + excluded.fail_count,
                last_used_ts = CASE
                    WHEN excluded.last_used_ts > last_used_ts THEN excluded.last_used_ts
                    ELSE last_used_ts
                END
            RETURNING id
            "#,
        )
        .bind(key)
        .bind(root)
        .bind(sub)
        .bind(success_inc)
        .bind(fail_inc)
        .bind(used_ts as i64)
        .fetch_one(self.pool())
        .await?;

        Ok(id)
    }

    pub async fn upsert_transition(
        &self,
        prev_id: i64,
        next_id: i64,
        used_ts: u64,
        success: Option<bool>,
    ) -> RepositoryResult<()> {
        let (success_inc, fail_inc) = match success {
            Some(true) => (1_i64, 0_i64),
            Some(false) => (0_i64, 1_i64),
            None => (0_i64, 0_i64),
        };

        sqlx::query(
            r#"
            INSERT INTO completion_transitions
                (prev_id, next_id, count, success_count, fail_count, last_used_ts)
            VALUES
                (?, ?, 1, ?, ?, ?)
            ON CONFLICT(prev_id, next_id) DO UPDATE SET
                count = count + 1,
                success_count = success_count + excluded.success_count,
                fail_count = fail_count + excluded.fail_count,
                last_used_ts = CASE
                    WHEN excluded.last_used_ts > last_used_ts THEN excluded.last_used_ts
                    ELSE last_used_ts
                END
            "#,
        )
        .bind(prev_id)
        .bind(next_id)
        .bind(success_inc)
        .bind(fail_inc)
        .bind(used_ts as i64)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn upsert_entity(
        &self,
        entity_type: &str,
        value: &str,
        used_ts: u64,
    ) -> RepositoryResult<()> {
        sqlx::query(
            r#"
            INSERT INTO completion_entity_stats
                (entity_type, value, use_count, last_used_ts)
            VALUES
                (?, ?, 1, ?)
            ON CONFLICT(entity_type, value) DO UPDATE SET
                use_count = use_count + 1,
                last_used_ts = CASE
                    WHEN excluded.last_used_ts > last_used_ts THEN excluded.last_used_ts
                    ELSE last_used_ts
                END
            "#,
        )
        .bind(entity_type)
        .bind(value)
        .bind(used_ts as i64)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn top_next_keys(
        &self,
        prev_id: i64,
        limit: i64,
    ) -> RepositoryResult<Vec<(String, i64, i64, i64)>> {
        // (key, count, success_count, last_used_ts)
        let rows = sqlx::query_as::<_, (String, i64, i64, i64)>(
            r#"
            SELECT k.key, t.count, t.success_count, t.last_used_ts
            FROM completion_transitions t
            JOIN completion_command_keys k ON k.id = t.next_id
            WHERE t.prev_id = ?
            ORDER BY t.count DESC, t.last_used_ts DESC
            LIMIT ?
            "#,
        )
        .bind(prev_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }

    pub async fn find_key_id(&self, key: &str) -> RepositoryResult<Option<i64>> {
        let id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM completion_command_keys WHERE key = ? LIMIT 1",
        )
        .bind(key)
        .fetch_optional(self.pool())
        .await?;

        Ok(id)
    }

    pub async fn top_commands_by_frecency(
        &self,
        prefix: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<(String, i64, i64, i64)>> {
        // (key, use_count, success_count, last_used_ts)
        let rows = sqlx::query_as::<_, (String, i64, i64, i64)>(
            r#"
            SELECT key, use_count, success_count, last_used_ts
            FROM completion_command_keys
            WHERE key LIKE (? || '%')
            ORDER BY last_used_ts DESC, use_count DESC
            LIMIT ?
            "#,
        )
        .bind(prefix)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }

    pub async fn prune_older_than(&self, cutoff_ts: u64) -> RepositoryResult<()> {
        let cutoff = cutoff_ts as i64;

        sqlx::query("DELETE FROM completion_transitions WHERE last_used_ts < ?")
            .bind(cutoff)
            .execute(self.pool())
            .await?;

        sqlx::query("DELETE FROM completion_entity_stats WHERE last_used_ts < ?")
            .bind(cutoff)
            .execute(self.pool())
            .await?;

        // 删除不再被 transition 引用的老 key
        sqlx::query(
            r#"
            DELETE FROM completion_command_keys
            WHERE last_used_ts < ?
              AND id NOT IN (SELECT prev_id FROM completion_transitions)
              AND id NOT IN (SELECT next_id FROM completion_transitions)
            "#,
        )
        .bind(cutoff)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn enforce_transition_top_k_per_prev(
        &self,
        prev_id: i64,
        keep_k: i64,
    ) -> RepositoryResult<()> {
        // 只保留每个 prev_id 下最常用且最近的 K 条边
        sqlx::query(
            r#"
            DELETE FROM completion_transitions
            WHERE prev_id = ?
              AND (prev_id, next_id) NOT IN (
                SELECT prev_id, next_id
                FROM completion_transitions
                WHERE prev_id = ?
                ORDER BY count DESC, last_used_ts DESC
                LIMIT ?
              )
            "#,
        )
        .bind(prev_id)
        .bind(prev_id)
        .bind(keep_k)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn enforce_command_key_limit(&self, max_keys: i64) -> RepositoryResult<()> {
        // 删除最久未使用且不被引用的 key，直到数量 <= max_keys
        sqlx::query(
            r#"
            DELETE FROM completion_command_keys
            WHERE id IN (
                SELECT id
                FROM completion_command_keys
                WHERE id NOT IN (SELECT prev_id FROM completion_transitions)
                  AND id NOT IN (SELECT next_id FROM completion_transitions)
                ORDER BY last_used_ts ASC
                LIMIT (
                    SELECT CASE
                        WHEN COUNT(*) > ? THEN COUNT(*) - ?
                        ELSE 0
                    END
                    FROM completion_command_keys
                )
            )
            "#,
        )
        .bind(max_keys)
        .bind(max_keys)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
