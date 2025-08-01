/*!
 * 存储协调器模块
 *
 * 简化的存储协调器，管理三层存储的交互和数据流
 * 集成现有的配置管理器、MessagePack状态层和SQLite数据层
 * 遵循"不过度工程化"原则，只保留核心协调功能
 */

use crate::config::TomlConfigManager;
use crate::storage::messagepack::{MessagePackManager, MessagePackOptions};
use crate::storage::paths::StoragePaths;
use crate::storage::sqlite::{SqliteManager, SqliteOptions};
use crate::storage::types::{CacheStats, DataQuery, SaveOptions, SessionState, StorageStats};
use crate::storage::{CacheConfig, HealthCheckResult, MultiLayerCache, RecoveryManager};
use crate::utils::error::AppResult;
use anyhow::Context;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info};

/// 存储协调器选项
#[derive(Debug, Clone)]
pub struct StorageCoordinatorOptions {
    /// MessagePack管理器选项
    pub messagepack_options: MessagePackOptions,
    /// SQLite管理器选项
    pub sqlite_options: SqliteOptions,
}

impl Default for StorageCoordinatorOptions {
    fn default() -> Self {
        Self {
            messagepack_options: MessagePackOptions::default(),
            sqlite_options: SqliteOptions::default(),
        }
    }
}

/// 存储协调器
///
/// 简化的三层存储系统协调器，只保留核心功能
pub struct StorageCoordinator {
    /// 存储路径管理器
    paths: StoragePaths,
    /// 协调器选项
    options: StorageCoordinatorOptions,
    /// TOML配置管理器
    config_manager: Arc<TomlConfigManager>,
    /// MessagePack状态管理器
    messagepack_manager: Arc<MessagePackManager>,
    /// SQLite数据管理器
    sqlite_manager: Arc<SqliteManager>,
    /// 多层缓存管理器
    cache: Arc<MultiLayerCache>,
    /// 恢复管理器
    recovery_manager: Arc<RecoveryManager>,
}

impl StorageCoordinator {
    /// 创建新的存储协调器
    pub async fn new(
        paths: StoragePaths,
        options: StorageCoordinatorOptions,
        config_manager: Arc<TomlConfigManager>,
    ) -> AppResult<Self> {
        info!("初始化简化的存储协调器");

        // 确保所有目录存在
        paths.ensure_directories().context("创建存储目录失败")?;

        // 初始化MessagePack状态管理器
        let messagepack_manager = Arc::new(
            MessagePackManager::new(paths.clone(), options.messagepack_options.clone())
                .await
                .context("初始化MessagePack管理器失败")?,
        );

        // 初始化SQLite数据管理器
        let sqlite_manager = Arc::new(
            SqliteManager::new(paths.clone(), options.sqlite_options.clone())
                .await
                .context("初始化SQLite管理器失败")?,
        );

        // 初始化数据库
        sqlite_manager
            .initialize_database()
            .await
            .context("初始化数据库失败")?;

        // 初始化缓存系统
        let cache_config = CacheConfig::default();
        let cache = Arc::new(
            MultiLayerCache::new(&paths, cache_config)
                .await
                .context("初始化缓存系统失败")?,
        );

        // 初始化恢复管理器
        let recovery_manager = Arc::new(RecoveryManager::new(paths.clone()));

        let coordinator = Self {
            paths,
            options,
            config_manager,
            messagepack_manager,
            sqlite_manager,
            cache,
            recovery_manager,
        };

        info!("存储协调器初始化完成");
        Ok(coordinator)
    }

    /// 获取配置数据
    pub async fn get_config(&self, section: &str) -> AppResult<Value> {
        debug!("获取配置节: {}", section);

        // 从配置管理器获取配置（使用缓存）
        let config = self
            .config_manager
            .get_config()
            .await
            .context("获取配置失败")?;

        // 提取指定节的配置
        let section_value = match section {
            "app" => serde_json::to_value(&config.app)?,
            "appearance" => serde_json::to_value(&config.appearance)?,
            "terminal" => serde_json::to_value(&config.terminal)?,
            "ai" => {
                // AI配置已迁移到SQLite，返回空对象
                debug!("AI配置请求被重定向，AI配置已迁移到SQLite");
                Value::Object(serde_json::Map::new())
            }
            "shortcuts" => serde_json::to_value(&config.shortcuts)?,
            _ => Value::Object(serde_json::Map::new()),
        };

        Ok(section_value)
    }

    /// 更新配置数据
    pub async fn update_config(&self, section: &str, data: Value) -> AppResult<()> {
        debug!("更新配置节: {}", section);

        // 使用配置管理器的 update_section 方法
        self.config_manager
            .update_section(section, data)
            .await
            .context("更新配置节失败")?;

        Ok(())
    }

    /// 保存会话状态
    pub async fn save_session_state(&self, state: &SessionState) -> AppResult<()> {
        debug!("保存会话状态");
        self.messagepack_manager.save_state(state).await
    }

    /// 加载会话状态
    pub async fn load_session_state(&self) -> AppResult<Option<SessionState>> {
        debug!("加载会话状态");
        self.messagepack_manager.load_state().await
    }

    /// 查询数据
    pub async fn query_data(&self, query: &DataQuery) -> AppResult<Vec<Value>> {
        debug!("查询数据: {}", query.query);
        self.sqlite_manager.query_data(query).await
    }

    /// 保存数据
    pub async fn save_data(&self, data: &Value, options: &SaveOptions) -> AppResult<()> {
        debug!("保存数据到表: {:?}", options.table);
        self.sqlite_manager.save_data(data, options).await
    }

    /// 获取存储路径管理器的引用
    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    /// 获取协调器选项的引用
    pub fn options(&self) -> &StorageCoordinatorOptions {
        &self.options
    }

    /// 获取SQLite管理器的引用
    pub fn sqlite_manager(&self) -> Arc<SqliteManager> {
        self.sqlite_manager.clone()
    }

    /// 健康检查
    pub async fn health_check(&self) -> AppResult<HealthCheckResult> {
        debug!("执行存储系统健康检查");
        let system_health = self.recovery_manager.health_check().await?;

        // 转换SystemHealth为HealthCheckResult
        Ok(HealthCheckResult {
            name: "storage_system".to_string(),
            healthy: system_health.overall_healthy,
            message: if system_health.overall_healthy {
                "存储系统健康".to_string()
            } else {
                format!(
                    "存储系统存在问题: {} 个检查项失败",
                    system_health.unhealthy_checks().len()
                )
            },
            checked_at: system_health.checked_at,
            duration: system_health.total_duration,
        })
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> AppResult<CacheStats> {
        debug!("获取缓存统计信息");
        Ok(self.cache.get_stats().await)
    }

    /// 获取存储统计信息
    pub async fn get_storage_stats(&self) -> AppResult<StorageStats> {
        debug!("获取存储统计信息");

        let cache_stats = self.cache.get_stats().await;
        let config_size = self
            .paths
            .config_file()
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let state_size = self
            .paths
            .session_state_file()
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let db_size = self
            .paths
            .database_file()
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(StorageStats {
            total_size: config_size + state_size + db_size,
            config_size,
            state_size,
            data_size: db_size,
            cache_size: cache_stats.total_memory_usage,
            backups_size: 0, // 简化实现
            logs_size: 0,    // 简化实现
        })
    }

    /// 预加载缓存
    pub async fn preload_cache(&self) -> AppResult<()> {
        debug!("预加载缓存");

        // 预加载配置数据
        if let Ok(config) = self.config_manager.load_config().await {
            let config_value = serde_json::to_value(&config)?;
            self.cache.set("config:all", config_value).await?;
        }

        // 预加载会话状态
        if let Ok(Some(state)) = self.messagepack_manager.load_state().await {
            let state_value = serde_json::to_value(&state)?;
            self.cache.set("session:state", state_value).await?;
        }

        info!("缓存预加载完成");
        Ok(())
    }

    /// 清空缓存
    pub async fn clear_cache(&self) -> AppResult<()> {
        debug!("清空缓存");
        self.cache.clear().await?;
        info!("缓存已清空");
        Ok(())
    }
}
