/*!
 * 向量索引配置服务
 *
 * 提供向量索引配置的管理服务，包括保存、加载、验证等功能。
 * 遵循Repository模式，通过AIFeaturesRepository进行数据持久化。
 */

use crate::storage::repositories::{
    ai_features::AIFeatureConfig, AIFeaturesRepository, RepositoryManager,
};
use crate::utils::error::AppResult;
use crate::vector_index::types::VectorIndexConfig;
use anyhow::{anyhow, ensure, Context};
use std::sync::Arc;
use tracing::{debug, info};

/// 向量索引配置的功能名称常量
const VECTOR_INDEX_FEATURE_NAME: &str = "vector_index";

/// 向量索引配置服务
pub struct VectorIndexConfigService {
    repository_manager: Arc<RepositoryManager>,
}

impl VectorIndexConfigService {
    /// 创建新的向量索引配置服务
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    /// 保存向量索引配置
    pub async fn save_config(&self, config: &VectorIndexConfig) -> AppResult<()> {
        info!("保存向量索引配置");

        // 1. 配置验证
        self.validate_config(config)?;

        // 2. 创建AI功能配置实体
        let feature_config = AIFeatureConfig::from_config(
            VECTOR_INDEX_FEATURE_NAME.to_string(),
            true, // 默认启用
            config,
        )
        .context("创建AI功能配置失败")?;

        // 3. 保存到数据库
        self.repository_manager
            .ai_features()
            .save_or_update(&feature_config)
            .await
            .context("保存向量索引配置到数据库失败")?;

        info!("向量索引配置保存成功");
        Ok(())
    }

    /// 加载向量索引配置
    pub async fn load_config(&self) -> AppResult<Option<VectorIndexConfig>> {
        debug!("加载向量索引配置");

        let feature_config_opt = self
            .repository_manager
            .ai_features()
            .find_by_feature_name(VECTOR_INDEX_FEATURE_NAME)
            .await
            .context("从数据库查询向量索引配置失败")?;

        match feature_config_opt {
            Some(feature_config) => {
                debug!("找到向量索引配置");

                let config = feature_config
                    .parse_config::<VectorIndexConfig>()
                    .context("解析向量索引配置JSON失败")?;

                match config {
                    Some(parsed_config) => {
                        debug!("向量索引配置加载成功");
                        Ok(Some(parsed_config))
                    }
                    None => {
                        debug!("向量索引配置为空");
                        Ok(None)
                    }
                }
            }
            None => {
                debug!("未找到向量索引配置，将使用默认配置");
                Ok(None)
            }
        }
    }

    /// 删除向量索引配置
    pub async fn delete_config(&self) -> AppResult<()> {
        info!("删除向量索引配置");

        self.repository_manager
            .ai_features()
            .delete_by_feature_name(VECTOR_INDEX_FEATURE_NAME)
            .await
            .context("从数据库删除向量索引配置失败")?;

        info!("向量索引配置删除成功");
        Ok(())
    }

    /// 检查向量索引配置是否存在
    pub async fn config_exists(&self) -> AppResult<bool> {
        debug!("检查向量索引配置是否存在");

        let exists = self
            .repository_manager
            .ai_features()
            .find_by_feature_name(VECTOR_INDEX_FEATURE_NAME)
            .await
            .context("查询向量索引配置存在性失败")?
            .is_some();

        debug!("向量索引配置存在状态: {}", exists);
        Ok(exists)
    }

    /// 启用或禁用向量索引功能
    pub async fn set_feature_enabled(&self, enabled: bool) -> AppResult<()> {
        info!("设置向量索引功能状态: {}", enabled);

        let mut feature_config = self
            .repository_manager
            .ai_features()
            .find_by_feature_name(VECTOR_INDEX_FEATURE_NAME)
            .await
            .context("查询向量索引配置失败")?
            .ok_or_else(|| anyhow!("向量索引配置不存在，无法设置功能状态"))?;

        feature_config.enabled = enabled;

        self.repository_manager
            .ai_features()
            .save_or_update(&feature_config)
            .await
            .context("更新向量索引功能状态失败")?;

        info!("向量索引功能状态更新成功: {}", enabled);
        Ok(())
    }

    /// 获取向量索引功能是否启用
    pub async fn is_feature_enabled(&self) -> AppResult<bool> {
        debug!("查询向量索引功能启用状态");

        let feature_config_opt = self
            .repository_manager
            .ai_features()
            .find_by_feature_name(VECTOR_INDEX_FEATURE_NAME)
            .await
            .context("查询向量索引配置失败")?;

        let enabled = feature_config_opt
            .map(|config| config.enabled)
            .unwrap_or(false);

        debug!("向量索引功能启用状态: {}", enabled);
        Ok(enabled)
    }

    /// 获取向量索引配置或默认配置
    pub async fn get_config_or_default(&self) -> AppResult<VectorIndexConfig> {
        match self.load_config().await? {
            Some(config) => Ok(config),
            None => {
                debug!("使用默认向量索引配置");
                Ok(VectorIndexConfig::default())
            }
        }
    }

    /// 验证向量索引配置
    fn validate_config(&self, config: &VectorIndexConfig) -> AppResult<()> {
        // 1. 验证Qdrant URL
        ensure!(
            !config.qdrant_url.trim().is_empty(),
            "Qdrant数据库URL不能为空"
        );

        // 2. 验证集合名称
        ensure!(
            !config.collection_name.trim().is_empty(),
            "向量集合名称不能为空"
        );

        // 3. 验证embedding模型ID
        ensure!(
            !config.embedding_model_id.trim().is_empty(),
            "Embedding模型ID不能为空"
        );

        // 4. 验证并发文件数
        ensure!(
            config.max_concurrent_files > 0 && config.max_concurrent_files <= 32,
            "最大并发文件数必须在1-32之间"
        );

        // 5. 验证URL格式（基础检查）
        if !config.qdrant_url.starts_with("http://") && !config.qdrant_url.starts_with("https://") {
            return Err(anyhow!("Qdrant URL格式不正确，必须以http://或https://开头"));
        }

        debug!("向量索引配置验证通过");
        Ok(())
    }

    /// 获取AI功能配置Repository（供高级操作使用）
    pub fn ai_features_repository(&self) -> &AIFeaturesRepository {
        self.repository_manager.ai_features()
    }
}
