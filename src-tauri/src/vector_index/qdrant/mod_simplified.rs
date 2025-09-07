/*!
 * Qdrant数据库集成模块 - 简化版本
 *
 * 临时版本：移除复杂的过滤功能，确保基本搜索和存储功能可以工作
 * TODO: 参考qdrant-client文档实现完整的API功能
 */

use crate::vector_index::types::{CodeVector, SearchOptions, SearchResult, VectorIndexConfig};
use anyhow::{Context, Result};
use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, ScoredPoint, SearchPointsBuilder,
        UpsertPointsBuilder, Value, VectorParamsBuilder, VectorsConfig,
    },
    Qdrant,
};
use std::collections::HashMap;
use std::time::Duration;

/// Qdrant服务接口
pub trait QdrantService {
    /// 测试数据库连接
    async fn test_connection(&self) -> Result<String>;

    /// 初始化向量集合
    async fn initialize_collection(&self) -> Result<()>;

    /// 批量上传向量
    async fn upload_vectors(&self, vectors: Vec<CodeVector>) -> Result<()>;

    /// 搜索相似向量
    async fn search_vectors(&self, options: SearchOptions) -> Result<Vec<SearchResult>>;

    /// 使用已向量化的查询进行搜索
    async fn search_with_vector(
        &self,
        query_vector: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>>;

    /// 删除指定文件的向量
    async fn delete_file_vectors(&self, file_path: &str) -> Result<()>;

    /// 获取集合统计信息
    async fn get_collection_info(&self) -> Result<(usize, usize)>; // (点数量, 向量数量)

    /// 清空所有向量数据
    async fn clear_all_vectors(&self) -> Result<()>;
}

/// Qdrant客户端实现 - 简化版本
pub struct QdrantClientImpl {
    client: Qdrant,
    config: VectorIndexConfig,
}

impl QdrantClientImpl {
    /// 创建新的Qdrant客户端
    pub async fn new(config: VectorIndexConfig) -> Result<Self> {
        tracing::info!("正在连接Qdrant数据库: {}", config.qdrant_url);

        // 创建Qdrant客户端
        let mut client_builder = Qdrant::from_url(&config.qdrant_url);

        // 设置API密钥（如果有）
        if let Some(api_key) = &config.qdrant_api_key {
            client_builder = client_builder.api_key(api_key.clone());
        }

        // 设置连接超时
        client_builder = client_builder.timeout(Duration::from_secs(30));

        let client = client_builder.build().context("创建Qdrant客户端失败")?;

        let instance = Self { client, config };

        // 测试连接
        instance
            .test_connection_internal()
            .await
            .context("Qdrant连接验证失败")?;

        tracing::info!("Qdrant客户端初始化成功");

        Ok(instance)
    }

    /// 内部连接测试（不返回字符串消息）
    async fn test_connection_internal(&self) -> Result<()> {
        // 通过获取集合列表来测试连接
        self.client
            .list_collections()
            .await
            .context("无法连接到Qdrant数据库")?;

        Ok(())
    }
}

impl QdrantService for QdrantClientImpl {
    async fn test_connection(&self) -> Result<String> {
        tracing::debug!("测试Qdrant数据库连接");

        match self.test_connection_internal().await {
            Ok(_) => {
                let message = format!("✅ Qdrant连接成功: {}", self.config.qdrant_url);
                tracing::info!("{}", message);
                Ok(message)
            }
            Err(e) => {
                let error_msg = format!("❌ Qdrant连接失败: {}", e);
                tracing::error!("{}", error_msg);
                Err(e)
            }
        }
    }

    async fn initialize_collection(&self) -> Result<()> {
        let collection_name = &self.config.collection_name;

        tracing::info!("初始化Qdrant集合: {}", collection_name);

        // 检查集合是否已存在
        let collections = self
            .client
            .list_collections()
            .await
            .context("获取集合列表失败")?;

        let collection_exists = collections
            .collections
            .iter()
            .any(|c| c.name == *collection_name);

        if collection_exists {
            tracing::info!("集合 '{}' 已存在", collection_name);
        } else {
            tracing::info!("创建新集合: {}", collection_name);
            self.create_new_collection().await?;
        }

        tracing::info!("集合初始化完成: {}", collection_name);
        Ok(())
    }

    async fn upload_vectors(&self, vectors: Vec<CodeVector>) -> Result<()> {
        if vectors.is_empty() {
            tracing::warn!("尝试上传空向量列表，跳过");
            return Ok(());
        }

        let collection_name = &self.config.collection_name;
        let total_vectors = vectors.len();

        tracing::info!(
            "开始上传 {} 个向量到Qdrant集合: {}",
            total_vectors,
            collection_name
        );

        // 使用适合Qdrant的批量大小
        let batch_size = crate::vector_index::constants::QDRANT_BATCH_SIZE; // 1000

        let mut uploaded_count = 0;

        for (batch_index, batch) in vectors.chunks(batch_size).enumerate() {
            tracing::debug!("上传批次 {} ({} 个向量)", batch_index + 1, batch.len());

            match self.upload_batch(batch).await {
                Ok(_) => {
                    uploaded_count += batch.len();
                    tracing::debug!("批次 {} 上传成功", batch_index + 1);
                }
                Err(e) => {
                    tracing::error!("批次 {} 上传失败: {}", batch_index + 1, e);
                    return Err(e).context(format!("批次 {} 上传失败", batch_index + 1));
                }
            }
        }

        tracing::info!("所有 {} 个向量上传成功", uploaded_count);
        Ok(())
    }

    async fn search_vectors(&self, _options: SearchOptions) -> Result<Vec<SearchResult>> {
        // 提示使用search_with_vector方法
        anyhow::bail!("请使用search_with_vector方法，并由VectorIndexService负责向量化查询文本");
    }

    async fn search_with_vector(
        &self,
        query_vector: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        tracing::info!(
            "开始向量搜索: '{}' (最多{}个结果)",
            options.query,
            options.max_results.unwrap_or(10)
        );

        // 验证查询向量维度
        anyhow::ensure!(
            query_vector.len() == self.config.vector_size,
            "查询向量维度不匹配: 期望 {}, 实际 {}",
            self.config.vector_size,
            query_vector.len()
        );

        // 构建简化的搜索请求（不使用过滤）
        let search_request = SearchPointsBuilder::new(
            &self.config.collection_name,
            query_vector,
            options.max_results.unwrap_or(10) as u64,
        )
        .score_threshold(options.min_score.unwrap_or(0.3))
        .with_payload(true)
        .build();

        // 执行搜索
        let search_response = self
            .client
            .search_points(search_request)
            .await
            .context("Qdrant向量搜索失败")?;

        // 处理和格式化结果
        let results = self.format_search_results(search_response.result)?;

        tracing::info!("搜索完成: 找到 {} 个相关结果", results.len());
        Ok(results)
    }

    async fn delete_file_vectors(&self, file_path: &str) -> Result<()> {
        tracing::info!("删除文件向量: {}", file_path);

        // TODO: 实现文件向量删除功能
        // 需要参考qdrant-client文档实现正确的过滤删除
        tracing::warn!("文件向量删除功能暂时禁用，需要参考文档实现正确的API调用");

        Ok(())
    }

    async fn get_collection_info(&self) -> Result<(usize, usize)> {
        let collection_name = &self.config.collection_name;

        tracing::debug!("获取集合信息: {}", collection_name);

        // 获取集合详细信息
        let collection_info = self
            .client
            .collection_info(collection_name)
            .await
            .context("获取集合信息失败")?;

        // 从集合信息中获取点数量
        let points_count = if let Some(result) = collection_info.result {
            result.points_count.unwrap_or(0) as usize
        } else {
            0
        };

        let vectors_count = points_count; // 假设每个点只有一个向量

        tracing::debug!(
            "集合 '{}' 统计信息: {} 个点, {} 个向量",
            collection_name,
            points_count,
            vectors_count
        );

        Ok((points_count, vectors_count))
    }

    async fn clear_all_vectors(&self) -> Result<()> {
        let collection_name = &self.config.collection_name;

        tracing::info!("开始清空集合中的所有向量: {}", collection_name);

        // 删除现有集合
        match self.client.delete_collection(collection_name).await {
            Ok(_) => {
                tracing::info!("成功删除集合: {}", collection_name);
            }
            Err(e) => {
                // 如果集合不存在，继续执行
                tracing::warn!("删除集合时遇到错误（可能不存在）: {}", e);
            }
        }

        // 重新创建空集合
        self.create_new_collection()
            .await
            .context("重新创建集合失败")?;

        tracing::info!("成功清空所有向量数据: {}", collection_name);
        Ok(())
    }
}

impl QdrantClientImpl {
    /// 创建新集合
    async fn create_new_collection(&self) -> Result<()> {
        let collection_name = &self.config.collection_name;

        // 构建向量参数
        let vector_params = VectorParamsBuilder::new(
            self.config.vector_size as u64,
            Distance::Cosine, // 使用余弦相似度
        )
        .on_disk(true) // 启用磁盘存储以支持大规模数据
        .build();

        // 构建向量配置
        let vectors_config = VectorsConfig {
            config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                vector_params,
            )),
        };

        // 创建集合
        let create_collection = CreateCollectionBuilder::new(collection_name)
            .vectors_config(vectors_config)
            .timeout(60)
            .build();

        self.client
            .create_collection(create_collection)
            .await
            .context("创建Qdrant集合失败")?;

        tracing::info!("成功创建Qdrant集合: {}", collection_name);
        Ok(())
    }

    /// 批量上传单个批次的向量
    async fn upload_batch(&self, vectors: &[CodeVector]) -> Result<()> {
        let collection_name = &self.config.collection_name;

        // 将CodeVector转换为Qdrant的PointStruct
        let points: Vec<PointStruct> = vectors
            .iter()
            .map(|vector| self.code_vector_to_point(vector))
            .collect::<Result<Vec<_>>>()?;

        // 构建upsert请求
        let upsert_request = UpsertPointsBuilder::new(collection_name, points)
            .wait(true)
            .build();

        // 执行upsert操作
        self.client
            .upsert_points(upsert_request)
            .await
            .context("执行向量upsert操作失败")?;

        Ok(())
    }

    /// 将CodeVector转换为Qdrant的PointStruct
    fn code_vector_to_point(&self, vector: &CodeVector) -> Result<PointStruct> {
        // 构建payload - 包含代码的所有元数据
        let mut payload = HashMap::new();

        // 基本文件信息
        payload.insert(
            "file_path".to_string(),
            Value::from(vector.file_path.clone()),
        );
        payload.insert("content".to_string(), Value::from(vector.content.clone()));
        payload.insert(
            "start_line".to_string(),
            Value::from(vector.start_line as i64),
        );
        payload.insert("end_line".to_string(), Value::from(vector.end_line as i64));
        payload.insert("language".to_string(), Value::from(vector.language.clone()));
        payload.insert(
            "chunk_type".to_string(),
            Value::from(vector.chunk_type.clone()),
        );

        // 添加额外的元数据
        for (key, value) in &vector.metadata {
            payload.insert(format!("meta_{}", key), Value::from(value.clone()));
        }

        // 验证向量维度
        let expected_size = self.config.vector_size;
        anyhow::ensure!(
            vector.vector.len() == expected_size,
            "向量维度不匹配: 期望 {}, 实际 {}",
            expected_size,
            vector.vector.len()
        );

        // 创建PointStruct
        Ok(PointStruct::new(
            vector.id.clone(),
            vector.vector.clone(),
            payload,
        ))
    }

    /// 格式化搜索结果
    fn format_search_results(&self, scored_points: Vec<ScoredPoint>) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        for scored_point in scored_points {
            let payload = scored_point.payload;

            // 提取基本字段
            let file_path = payload
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let start_line = payload
                .get("start_line")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u32;

            let end_line = payload
                .get("end_line")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u32;

            let language = payload
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let chunk_type = payload
                .get("chunk_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 提取元数据
            let mut metadata = HashMap::new();
            for (key, value) in payload.iter() {
                if key.starts_with("meta_") {
                    let meta_key = key.strip_prefix("meta_").unwrap().to_string();
                    let meta_value = value.as_str().unwrap_or("").to_string();
                    metadata.insert(meta_key, meta_value);
                }
            }

            let result = SearchResult {
                id: scored_point
                    .id
                    .map(|id| format!("{:?}", id))
                    .unwrap_or_default(),
                file_path,
                content,
                start_line,
                end_line,
                language,
                chunk_type,
                score: scored_point.score,
                metadata,
            };

            results.push(result);
        }

        tracing::debug!("格式化了 {} 个搜索结果", results.len());
        Ok(results)
    }
}
