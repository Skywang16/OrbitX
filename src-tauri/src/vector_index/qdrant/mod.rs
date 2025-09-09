/*!
 * Qdrant数据库集成模块
 *
 * 提供向量数据库的连接、存储和搜索功能。
 * 支持批量操作和连接管理。
 *
 * Requirements: 3.1, 3.2, 3.3, 3.4
 *
 * 说明：本文件为唯一且权威的 Qdrant 实现入口。为降低阅读成本，简化版实现
 * `mod_simplified.rs` 已被移除/隔离，不再与本实现并存。
 */

use crate::vector_index::types::{CodeVector, SearchOptions, SearchResult, VectorIndexFullConfig};
use anyhow::{bail, ensure, Context, Result};
use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, Value,
        VectorParamsBuilder, VectorsConfig,
    },
    Qdrant,
};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

/// Qdrant服务接口
pub trait QdrantService {
    /// 测试数据库连接
    fn test_connection(&self) -> impl std::future::Future<Output = Result<String>> + Send;

    /// 初始化向量集合
    fn initialize_collection(&self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// 批量上传向量
    fn upload_vectors(
        &self,
        vectors: Vec<CodeVector>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// 搜索相似向量
    fn search_vectors(
        &self,
        options: SearchOptions,
    ) -> impl std::future::Future<Output = Result<Vec<SearchResult>>> + Send;

    /// 使用向量进行搜索（内部方法）
    fn search_with_vector(
        &self,
        query_vector: Vec<f32>,
        options: SearchOptions,
    ) -> impl std::future::Future<Output = Result<Vec<SearchResult>>> + Send;

    /// 删除指定文件的向量
    fn delete_file_vectors(
        &self,
        file_path: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// 获取集合统计信息
    fn get_collection_info(
        &self,
    ) -> impl std::future::Future<Output = Result<(usize, usize)>> + Send; // (点数量, 向量数量)

    /// 清空所有向量数据
    fn clear_all_vectors(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Qdrant客户端实现
pub struct QdrantClientImpl {
    client: Qdrant,
    config: VectorIndexFullConfig,
}

impl QdrantClientImpl {
    /// 创建新的Qdrant客户端
    pub async fn new(config: VectorIndexFullConfig) -> Result<Self> {
        tracing::info!("正在连接Qdrant数据库: {}", config.user_config.qdrant_url);

        // 在创建客户端之前，校验端点是否与 gRPC 客户端匹配
        Self::validate_endpoint(&config)?;

        // 创建Qdrant客户端
        let mut client_builder = Qdrant::from_url(&config.user_config.qdrant_url);

        // 设置API密钥（如果有）
        if let Some(api_key) = &config.user_config.qdrant_api_key {
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

impl QdrantClientImpl {
    /// 校验配置的 Qdrant 端点，确保与 gRPC 客户端匹配
    fn validate_endpoint(config: &VectorIndexFullConfig) -> Result<()> {
        let url = Url::parse(&config.user_config.qdrant_url)
            .with_context(|| format!("无法解析Qdrant地址: {}", config.user_config.qdrant_url))?;

        let host = url.host_str().unwrap_or("");
        let scheme = url.scheme();
        let port = url.port_or_known_default();

        // 1) 明确禁止将 gRPC 客户端指向 6333（REST 端口）
        if let Some(p) = port {
            if p == 6333 {
                bail!(
                    "检测到将gRPC客户端连接到REST端口 6333。请将端口改为 6334。示例：\n- 本地: http://localhost:6334\n- 云端: https://<cluster>.<region>.<zone>.aws.cloud.qdrant.io:6334"
                );
            }
        }

        // 2) 云端域名的特殊提示：必须使用 6334 且建议 https
        let is_cloud = host.ends_with(".cloud.qdrant.io") || host.ends_with(".qdrant.io");
        if is_cloud {
            // 云端强烈建议使用 https
            if scheme != "https" {
                tracing::warn!(
                    "Qdrant Cloud 建议使用 HTTPS。当前为 '{}', 建议切换为 'https'。",
                    scheme
                );
            }

            // 云端 gRPC 端口必须为 6334
            if port != Some(6334) {
                bail!(
                    "Qdrant Cloud gRPC 端点应使用端口 6334。请将地址改为形如：https://{}:6334",
                    host
                );
            }
        }

        Ok(())
    }
}

impl QdrantService for QdrantClientImpl {
    async fn test_connection(&self) -> Result<String> {
        tracing::debug!("测试Qdrant数据库连接");

        match self.test_connection_internal().await {
            Ok(_) => {
                let message = format!("✅ Qdrant连接成功: {}", self.config.user_config.qdrant_url);
                tracing::info!("{}", message);
                Ok(message)
            }
            Err(e) => {
                let error_msg = format!("❌ Qdrant连接失败: {}", e);
                tracing::error!("{}", error_msg);
                bail!(error_msg)
            }
        }
    }

    async fn initialize_collection(&self) -> Result<()> {
        let collection_name = &self.config.user_config.collection_name;

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
            tracing::info!("集合 '{}' 已存在，验证配置", collection_name);

            // 验证现有集合的配置
            match self.validate_existing_collection().await {
                Ok(_) => {
                    tracing::info!("现有集合配置验证通过");
                }
                Err(e) => {
                    tracing::info!("集合配置不匹配({}), 重新创建", e);
                    
                    // 删除现有集合并重新创建
                    let _ = self.client.delete_collection(collection_name).await;
                    self.create_new_collection().await?;
                }
            }
        } else {
            tracing::info!("创建新集合: {}", collection_name);

            // 创建新集合
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

        let collection_name = &self.config.user_config.collection_name;
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

                    // 对于批量上传失败，我们采用失败则停止的策略以确保数据一致性
                    return Err(e).context(format!("批次 {} 上传失败", batch_index + 1));
                }
            }
        }

        tracing::info!("所有 {} 个向量上传成功", uploaded_count);

        Ok(())
    }

    async fn search_vectors(&self, options: SearchOptions) -> Result<Vec<SearchResult>> {
        // 注意：这个方法现在是临时实现，向量化应该在VectorIndexService中进行
        tracing::warn!(
            "search_vectors被调用：应该使用VectorIndexService.search_vectors进行完整的搜索流程"
        );

        // 使用占位符向量进行搜索（仅用于测试）
        let placeholder_vector = vec![0.0f32; self.config.vector_size()];
        self.search_with_vector(placeholder_vector, options).await
    }

    async fn search_with_vector(
        &self,
        query_vector: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        use qdrant_client::qdrant::SearchPointsBuilder;

        let collection_name = &self.config.user_config.collection_name;

        tracing::info!(
            "开始向量搜索: '{}' (collection: {})",
            options.query,
            collection_name
        );

        // 1. 输入验证
        ensure!(
            options.query.len() >= 3,
            "查询文本过短：需要至少3个字符，当前{}个字符",
            options.query.len()
        );

        ensure!(
            query_vector.len() == self.config.vector_size(),
            "查询向量维度不匹配：期望 {}, 实际 {}",
            self.config.vector_size(),
            query_vector.len()
        );

        // 2. 构建搜索请求
        let max_results = options
            .max_results
            .unwrap_or(crate::vector_index::constants::DEFAULT_SEARCH_RESULTS);
        let min_score = options
            .min_score
            .unwrap_or(crate::vector_index::constants::DEFAULT_MIN_SCORE);

        let mut search_builder =
            SearchPointsBuilder::new(collection_name, query_vector, max_results as u64)
                .score_threshold(min_score)
                .with_payload(true); // 包含所有payload数据

        // 3. 添加过滤条件
        if let Some(filter) = self.build_search_filters(&options)? {
            search_builder = search_builder.filter(filter);
        }

        let search_request = search_builder.build();

        // 4. 执行搜索
        tracing::debug!(
            "执行Qdrant搜索 (max_results: {}, min_score: {})",
            max_results,
            min_score
        );
        let search_response = self
            .client
            .search_points(search_request)
            .await
            .context("Qdrant向量搜索失败")?;

        // 5. 格式化结果
        let results = self.format_search_results(search_response.result)?;

        tracing::info!(
            "搜索完成: 查询='{}', 找到{}个结果 (阈值: {}, 最大: {})",
            options.query,
            results.len(),
            min_score,
            max_results
        );

        Ok(results)
    }

    async fn delete_file_vectors(&self, _file_path: &str) -> Result<()> {
        use qdrant_client::qdrant::{Condition, DeletePointsBuilder, Filter};

        let collection_name = &self.config.user_config.collection_name;

        // 构建按 file_path 精确匹配的过滤器
        let filter = Filter::must([Condition::matches("file_path", _file_path.to_string())]);

        tracing::info!(
            "删除集合 '{}' 中 file_path={} 的向量",
            collection_name,
            _file_path
        );

        let delete_req = DeletePointsBuilder::new(collection_name)
            .points(filter)
            .wait(true)
            .build();

        self.client
            .delete_points(delete_req)
            .await
            .context("删除文件向量失败")?;

        Ok(())
    }

    async fn get_collection_info(&self) -> Result<(usize, usize)> {
        let collection_name = &self.config.user_config.collection_name;

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
        let collection_name = &self.config.user_config.collection_name;

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
    /// 验证现有集合的配置
    async fn validate_existing_collection(&self) -> Result<()> {
        let collection_name = &self.config.user_config.collection_name;

        let collection_info = self
            .client
            .collection_info(collection_name)
            .await
            .context("获取现有集合信息失败")?;

        // 检查向量维度
        if let Some(result) = &collection_info.result {
            if let Some(config) = &result.config {
                if let Some(params) = &config.params {
                    if let Some(vectors_config) = &params.vectors_config {
                        if let Some(vectors_config_value) = &vectors_config.config {
                            match vectors_config_value {
                                qdrant_client::qdrant::vectors_config::Config::Params(
                                    vector_params,
                                ) => {
                                    let expected_size = self.config.vector_size() as u64;
                                    let actual_size = vector_params.size;

                                    ensure!(
                                        actual_size == expected_size,
                                        "集合向量维度不匹配: 期望 {}, 实际 {}",
                                        expected_size,
                                        actual_size
                                    );
                                }
                                qdrant_client::qdrant::vectors_config::Config::ParamsMap(_) => {
                                    bail!("不支持命名向量配置，请使用单一向量配置");
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("现有集合配置验证通过");
        Ok(())
    }

    /// 创建新集合
    async fn create_new_collection(&self) -> Result<()> {
        let collection_name = &self.config.user_config.collection_name;

        // 构建向量参数
        let vector_params = VectorParamsBuilder::new(
            self.config.vector_size() as u64,
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

        // 创建集合 - 使用简化的API
        let create_collection = CreateCollectionBuilder::new(collection_name)
            .vectors_config(vectors_config)
            .timeout(60) // 超时时间（秒）
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
        let collection_name = &self.config.user_config.collection_name;

        // 将CodeVector转换为Qdrant的PointStruct
        let points: Vec<PointStruct> = vectors
            .iter()
            .map(|vector| self.code_vector_to_point(vector))
            .collect::<Result<Vec<_>>>()?;

        // 构建upsert请求
        let upsert_request = UpsertPointsBuilder::new(collection_name, points)
            .wait(true) // 等待操作完成
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

        // 添加用于过滤的索引字段
        payload.insert(
            "file_dir".to_string(),
            Value::from(
                std::path::Path::new(&vector.file_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string(),
            ),
        );

        payload.insert(
            "file_name".to_string(),
            Value::from(
                std::path::Path::new(&vector.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
            ),
        );

        // 内容长度（用于过滤）
        payload.insert(
            "content_length".to_string(),
            Value::from(vector.content.len() as i64),
        );

        // 行数（用于过滤）
        payload.insert(
            "line_count".to_string(),
            Value::from((vector.end_line - vector.start_line + 1) as i64),
        );

        // 验证向量维度
        let expected_size = self.config.vector_size();
        ensure!(
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

    /// 构建搜索过滤条件
    fn build_search_filters(
        &self,
        options: &SearchOptions,
    ) -> Result<Option<qdrant_client::qdrant::Filter>> {
        use qdrant_client::qdrant::{Condition, FieldCondition, Filter, Match};

        let mut conditions = Vec::new();

        // 目录过滤
        if let Some(directory) = &options.directory_filter {
            tracing::debug!("添加目录过滤: {}", directory);

            let condition = Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "file_dir".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                directory.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            };
            conditions.push(condition);
        }

        // 语言过滤
        if let Some(language) = &options.language_filter {
            tracing::debug!("添加语言过滤: {}", language);

            let condition = Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "language".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                language.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            };
            conditions.push(condition);
        }

        // 代码块类型过滤
        if let Some(chunk_type) = &options.chunk_type_filter {
            tracing::debug!("添加代码块类型过滤: {}", chunk_type);

            let condition = Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "chunk_type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                chunk_type.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            };
            conditions.push(condition);
        }

        // 如果有过滤条件，构建Filter
        if !conditions.is_empty() {
            tracing::debug!("构建了 {} 个过滤条件", conditions.len());

            Ok(Some(Filter {
                must: conditions,
                ..Default::default()
            }))
        } else {
            Ok(None)
        }
    }

    /// 格式化搜索结果
    fn format_search_results(
        &self,
        scored_points: Vec<qdrant_client::qdrant::ScoredPoint>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        for scored_point in scored_points {
            let payload = scored_point.payload;

            // 提取基本字段
            let file_path = self.extract_string_field(&payload, "file_path")?;
            let content = self.extract_string_field(&payload, "content")?;
            let start_line = self.extract_integer_field(&payload, "start_line")? as u32;
            let end_line = self.extract_integer_field(&payload, "end_line")? as u32;
            let language = self.extract_string_field(&payload, "language")?;
            let chunk_type = self.extract_string_field(&payload, "chunk_type")?;

            // 提取元数据
            let mut metadata = HashMap::new();
            for (key, value) in &payload {
                if key.starts_with("meta_") {
                    if let Some(meta_key) = key.strip_prefix("meta_") {
                        if let Some(text_value) = value.as_str() {
                            metadata.insert(meta_key.to_string(), text_value.to_string());
                        }
                    }
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

        // 按分数降序排序
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        tracing::debug!("格式化了 {} 个搜索结果", results.len());
        Ok(results)
    }

    /// 从payload中提取字符串字段
    fn extract_string_field(
        &self,
        payload: &HashMap<String, qdrant_client::qdrant::Value>,
        field_name: &str,
    ) -> Result<String> {
        payload
            .get(field_name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("缺少或无效的字段: {}", field_name))
    }

    /// 从payload中提取整数字段
    fn extract_integer_field(
        &self,
        payload: &HashMap<String, qdrant_client::qdrant::Value>,
        field_name: &str,
    ) -> Result<i64> {
        payload
            .get(field_name)
            .and_then(|v| v.as_integer())
            .ok_or_else(|| anyhow::anyhow!("缺少或无效的整数字段: {}", field_name))
    }
}
