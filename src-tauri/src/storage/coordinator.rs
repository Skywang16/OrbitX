/*!
 * 存储协调器模块
 *
 * 管理三层存储架构的协调器，采用Repository模式
 * 集成配置管理器、MessagePack状态层和数据库层
 * 提供统一的存储访问接口
 */

use crate::config::TomlConfigManager;
use crate::storage::cache::UnifiedCache;
use crate::storage::database::{DatabaseManager, DatabaseOptions};
use crate::storage::messagepack::{MessagePackManager, MessagePackOptions};
use crate::storage::paths::StoragePaths;
use crate::storage::repositories::RepositoryManager;
use crate::storage::types::SessionState;

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
    /// 数据库管理器选项
    pub database_options: DatabaseOptions,
}

impl Default for StorageCoordinatorOptions {
    fn default() -> Self {
        Self {
            messagepack_options: MessagePackOptions::default(),
            database_options: DatabaseOptions::default(),
        }
    }
}

/// 存储协调器
///
/// 三层存储系统协调器，采用Repository模式管理数据访问
pub struct StorageCoordinator {
    /// 存储路径管理器
    paths: StoragePaths,
    /// 协调器选项
    options: StorageCoordinatorOptions,
    /// TOML配置管理器
    config_manager: Arc<TomlConfigManager>,
    /// MessagePack状态管理器
    messagepack_manager: Arc<MessagePackManager>,
    /// 数据库管理器
    database_manager: Arc<DatabaseManager>,
    /// Repository管理器
    repository_manager: Arc<RepositoryManager>,
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

        // 初始化数据库管理器
        let database_manager = Arc::new(
            DatabaseManager::new(paths.clone(), options.database_options.clone())
                .await
                .context("初始化数据库管理器失败")?,
        );

        // 初始化数据库
        database_manager
            .initialize()
            .await
            .context("初始化数据库失败")?;

        // 初始化Repository管理器
        let repository_manager = Arc::new(RepositoryManager::new(database_manager.clone()));



        let coordinator = Self {
            paths,
            options,
            config_manager,
            messagepack_manager,
            database_manager,
            repository_manager,
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



    /// 获取存储路径管理器的引用
    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    /// 获取协调器选项的引用
    pub fn options(&self) -> &StorageCoordinatorOptions {
        &self.options
    }

    /// 获取数据库管理器的引用
    pub fn database_manager(&self) -> Arc<DatabaseManager> {
        self.database_manager.clone()
    }

    /// 获取Repository管理器的引用
    pub fn repositories(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }

    /// 获取缓存管理器的引用
    pub fn cache(&self) -> Arc<UnifiedCache> {
        self.cache.clone()
    }
}
