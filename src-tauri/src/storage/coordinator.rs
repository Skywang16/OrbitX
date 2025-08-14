/*!
 * 存储协调器模块
 *
 * 简化的存储协调器，管理三层存储的交互和数据流
 * 集成现有的配置管理器、MessagePack状态层和SQLite数据层
 * 遵循"不过度工程化"原则，只保留核心协调功能
 */

use crate::config::TomlConfigManager;

use crate::storage::cache::UnifiedCache;
use crate::storage::messagepack::{MessagePackManager, MessagePackOptions};
use crate::storage::paths::StoragePaths;
use crate::storage::sqlite::{SqliteManager, SqliteOptions};
use crate::storage::types::{DataQuery, SaveOptions, SessionState};
use crate::storage::RecoveryManager;
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

    /// 恢复管理器
    recovery_manager: Arc<RecoveryManager>,

    /// 统一缓存
    cache: Arc<UnifiedCache>,
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

        // 初始化恢复管理器
        let recovery_manager = Arc::new(RecoveryManager::new(paths.clone()));

        let coordinator = Self {
            paths,
            options,
            config_manager,
            messagepack_manager,
            sqlite_manager,

            recovery_manager,
            cache: Arc::new(UnifiedCache::new()),
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

    /// 获取缓存管理器的引用
    pub fn cache(&self) -> Arc<UnifiedCache> {
        self.cache.clone()
    }
}
