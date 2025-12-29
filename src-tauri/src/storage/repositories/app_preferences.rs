/*!
 * 全局偏好设置存储
 *
 * 用于持久化项目/用户规则等简单的键值对
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::RepositoryResult;
use sqlx::Row;

pub struct AppPreferences<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AppPreferences<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.db.pool()
    }

    /// 获取指定键值
    pub async fn get(&self, key: &str) -> RepositoryResult<Option<String>> {
        let row = sqlx::query("SELECT value FROM app_preferences WHERE key = ? LIMIT 1")
            .bind(key)
            .fetch_optional(self.pool())
            .await?;

        Ok(row
            .and_then(|r| r.try_get::<Option<String>, _>("value").ok())
            .flatten())
    }

    /// 设置指定键值；当 value 为 None 时删除
    pub async fn set(&self, key: &str, value: Option<&str>) -> RepositoryResult<()> {
        match value {
            Some(v) => {
                sqlx::query(
                    r#"
                    INSERT INTO app_preferences (key, value, updated_at)
                    VALUES (?, ?, CURRENT_TIMESTAMP)
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                    "#,
                )
                .bind(key)
                .bind(v)
                .execute(self.pool())
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM app_preferences WHERE key = ?")
                    .bind(key)
                    .execute(self.pool())
                    .await?;
            }
        }

        Ok(())
    }
}
