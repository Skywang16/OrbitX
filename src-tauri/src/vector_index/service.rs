/*!
 * 向量索引服务模块
 *
 * 统一的向量索引服务，整合代码解析、向量化、存储等功能。
 * 基于OrbitX现有的EkoCore AI框架和Tauri+Rust架构，提供完整的
 * 代码向量索引和语义搜索功能。
 *
 * ## 主要功能
 *
 * - **代码解析**: 基于Tree-sitter的语法感知代码分块
 * - **向量化处理**: 集成现有LLM服务进行代码向量化
 * - **并发处理**: 支持大规模文件的并发处理和进度报告
 * - **向量存储**: 通过Qdrant进行高效的向量存储和搜索
 *
 * ## 设计原则
 *
 * - 重用现有LLM服务和配置管理系统
 * - 遵循OrbitX开发规范，使用anyhow进行错误处理
 * - 简单实用的架构，避免过度抽象
 * - 专注实际需求，确保代码可维护性
 *
 * Requirements: 2.1, 2.2, 2.4, 4.1, 4.2, 4.3, 4.4
 */

use std::sync::Arc;
use std::time::Instant;

use anyhow::{ensure, Context, Result};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::llm::service::LLMService;
use crate::vector_index::{
    parser::{CodeParser, TreeSitterParser},
    qdrant::{QdrantClientImpl, QdrantService},
    types::{
        CodeVector, IndexStats, SearchOptions, SearchResult, TaskProgress, VectorIndexFullConfig,
    },
    vectorizer::{LLMVectorizationService, VectorizationService},
};

/// 向量索引服务 - 主要业务逻辑
pub struct VectorIndexService {
    config: VectorIndexFullConfig,
    parser: TreeSitterParser,
    vectorizer: LLMVectorizationService,
    storage: QdrantClientImpl,
}

impl VectorIndexService {
    /// 创建新的向量索引服务
    pub async fn new(
        user_config: crate::vector_index::types::VectorIndexConfig,
        llm_service: Arc<LLMService>,
        embedding_model: String,
    ) -> Result<Self> {
        // 构建完整配置（用户配置 + 内部技术配置）
        let full_config =
            VectorIndexFullConfig::from_user_config_with_model_info(user_config, &embedding_model);

        // 初始化各个组件
        let parser = TreeSitterParser::new(full_config.clone()).context("初始化代码解析器失败")?;

        let vectorizer =
            LLMVectorizationService::new(llm_service, embedding_model).with_max_retries(3);

        let storage = QdrantClientImpl::new(full_config.clone())
            .await
            .context("初始化Qdrant客户端失败")?;

        // 初始化Qdrant集合
        storage
            .initialize_collection()
            .await
            .context("初始化Qdrant集合失败")?;

        Ok(Self {
            config: full_config,
            parser,
            vectorizer,
            storage,
        })
    }

    /// 构建代码向量索引
    pub async fn build_index(
        &self,
        workspace_path: &str,
        progress_sender: Option<mpsc::Sender<TaskProgress>>,
        cancel_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<IndexStats> {
        let start_time = Instant::now();
        let task_id = Uuid::new_v4().to_string();

        tracing::info!("开始构建向量索引: {}", workspace_path);

        // 1. 扫描代码文件
        let file_scanner = crate::vector_index::parser::CodeFileScanner::new(self.config.clone())?;
        let (files, _scan_stats) = file_scanner
            .scan_directory(workspace_path)
            .await
            .context("扫描代码文件失败")?;

        let total_files = files.len();
        let mut stats = IndexStats {
            total_files,
            total_chunks: 0,
            vectorized_chunks: 0,
            uploaded_vectors: 0,
            processing_time: 0,
            failed_files: Vec::new(),
            errors: Vec::new(),
        };

        tracing::info!("发现 {} 个代码文件", total_files);

        // 2. 实时进度：不再使用模拟进度，仅在批次完成后发送真实进度

        // 3. 并发处理文件（管线式：每批处理完成即上传，避免累计内存）
        let mut processed_files = 0;
        let mut error_files = 0usize;
        let mut last_error_detail: Option<String> = None;
        let batch_size = self.config.user_config.max_concurrent_files;

        for file_batch in files.chunks(batch_size) {
            // 检查是否需要取消
            if let Some(ref flag) = cancel_flag {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    tracing::info!("索引构建被用户取消");
                    return Err(anyhow::anyhow!("索引构建被用户取消"));
                }
            }

            let mut batch_tasks = Vec::new();

            // 并发处理一批文件
            for file_path in file_batch {
                let parser = &self.parser;
                let vectorizer = &self.vectorizer;
                let file_path = file_path.clone();

                let task = async move {
                    // 将结果包装为 (file_path, Result<Vec<CodeVector>>)
                    let res: Result<Vec<CodeVector>> = async {
                        // 解析文件
                        let parsed_code = parser.parse_file(&file_path).await?;

                        if parsed_code.chunks.is_empty() {
                            return Ok(Vec::new());
                        }

                        // 准备向量化数据
                        let texts: Vec<String> = parsed_code
                            .chunks
                            .iter()
                            .map(|chunk| {
                                format!(
                                    "// 文件: {}\n// 类型: {}\n{}",
                                    file_path,
                                    chunk.chunk_type.as_str(),
                                    chunk.content
                                )
                            })
                            .collect();

                        // 向量化
                        let embeddings = vectorizer.create_embeddings(&texts).await?;

                        // 构建向量对象
                        let mut vectors = Vec::new();
                        for (chunk, embedding) in parsed_code.chunks.iter().zip(embeddings) {
                            let vector = CodeVector {
                                id: Uuid::new_v4().to_string(),
                                file_path: file_path.clone(),
                                content: chunk.content.clone(),
                                start_line: chunk.start_line,
                                end_line: chunk.end_line,
                                language: crate::vector_index::types::Language::from_extension(
                                    std::path::Path::new(&file_path)
                                        .extension()
                                        .and_then(|ext| ext.to_str())
                                        .unwrap_or(""),
                                )
                                .unwrap_or(crate::vector_index::types::Language::TypeScript)
                                .as_str()
                                .to_string(),
                                chunk_type: chunk.chunk_type.as_str().to_string(),
                                vector: embedding,
                                metadata: chunk.metadata.clone(),
                            };
                            vectors.push(vector);
                        }

                        Ok(vectors)
                    }
                    .await;

                    (file_path, res)
                };

                batch_tasks.push(Box::pin(task));
            }

            // 等待批次完成
            let batch_results = futures::future::join_all(batch_tasks).await;

            // 本批向量累积，随后立即上传
            let mut batch_vectors: Vec<CodeVector> = Vec::new();

            for (file_path, res) in batch_results {
                match res {
                    Ok(vectors) => {
                        stats.total_chunks += vectors.len();
                        stats.vectorized_chunks += vectors.len();
                        batch_vectors.extend(vectors);
                        processed_files += 1;

                        tracing::debug!("文件处理完成: {}", file_path);
                    }
                    Err(e) => {
                        let error_msg = format!("文件处理失败: {}", e);
                        stats.errors.push(error_msg.clone());
                        stats.failed_files.push(file_path.clone());
                        tracing::error!("{} (file: {})", error_msg, file_path);
                        processed_files += 1; // 失败也计入已处理
                        error_files += 1;
                        last_error_detail = Some(e.to_string());
                    }
                }
            }

            // 失败过多时提前终止，避免长时间无效重试
            let too_many_errors = error_files >= 20;
            let error_ratio_high =
                processed_files > 0 && (error_files as f32 / processed_files as f32) > 0.3;
            if too_many_errors || error_ratio_high {
                let base = format!(
                    "构建中止：文件处理错误过多（{}/{}，错误率 {:.0}%）",
                    error_files,
                    processed_files,
                    (error_files as f32 / processed_files as f32) * 100.0
                );
                let detail = last_error_detail
                    .as_ref()
                    .map(|d| format!("；根因示例：{}", d))
                    .unwrap_or_default();
                return Err(anyhow::anyhow!(format!("{}{}", base, detail)));
            }

            // 本批上传，避免全量累加
            if !batch_vectors.is_empty() {
                tracing::info!("开始上传本批 {} 个向量到Qdrant", batch_vectors.len());

                match self.storage.upload_vectors(batch_vectors).await {
                    Ok(_) => {
                        // 本批上传成功，按批次向量数量累加
                        // 注意：vectorized_chunks 统计了到目前为止的总生成量，这里只按本批数量增加上传计数
                        // 以避免累计差值的歧义
                        // 由于 batch_vectors 已被 move，此处改为记录时已提前保存其长度
                        // 通过日志已输出具体数量
                        // 简化为与 total_chunks 的增量相同：
                        // 由于我们已经将 vectors 追加到 batch_vectors，len 即为本批数量
                        // 但 batch_vectors 已 move，这里我们在上传前未保存长度，故改为上方日志前取 len()
                        // 为保证一致性，上方已在日志中获取长度，这里按 processed_files 的本批差值无法可靠推算
                        // 因此简单起见，按上一条日志中的数量不可访问，这里退一步：保持按 total_chunks 的增量逻辑
                        // 然而此前实现易错，最终采用更直接方案：再次通过 stats.vectorized_chunks 与 uploaded_vectors 的差计算本批增量
                        // 注意：本方法在并发批处理中依然正确，因为每批结束时 vectorized_chunks 单调递增
                        stats.uploaded_vectors = stats.vectorized_chunks;
                        tracing::debug!("本批向量上传完成");
                    }
                    Err(e) => {
                        let error_msg = format!("向量上传失败: {}", e);
                        stats.errors.push(error_msg);
                        return Err(e);
                    }
                }
            }

            // 更新进度
            if let Some(sender) = &progress_sender {
                let progress = TaskProgress {
                    task_id: task_id.clone(),
                    progress: processed_files as f32 / total_files as f32,
                    status: format!("处理中 ({}/{})", processed_files, total_files),
                    current_file: None,
                    processed_files,
                    total_files,
                    cancellable: false,
                };
                let _ = sender.send(progress).await;
            }
        }

        // 5. 完成统计
        stats.processing_time = start_time.elapsed().as_millis() as u64;

        // 5.1 致命错误判定：如果所有文件都失败，或未生成任何向量，或错误列表包含关键致命错误关键词，则视为失败
        let all_failed = total_files > 0 && stats.failed_files.len() == total_files;
        let no_vectors = stats.vectorized_chunks == 0 || stats.uploaded_vectors == 0;
        let fatal_keywords = [
            "模型未找到",            // zh
            "model not found",       // en
            "解密",                  // zh for decryption
            "aead::Error",           // specific decrypt error from logs
            "Embedding API调用失败", // zh
            "embedding api",         // en
        ];
        let has_fatal_error = stats.errors.iter().any(|e| {
            fatal_keywords
                .iter()
                .any(|kw| e.to_lowercase().contains(&kw.to_lowercase()))
        });

        if all_failed || no_vectors || has_fatal_error {
            let reason = if all_failed {
                format!("所有文件处理失败（总计 {} 个）", total_files)
            } else if no_vectors {
                "未生成任何有效向量，可能是 Embedding 模型不可用或 API 密钥配置错误".to_string()
            } else {
                // 汇总致命错误信息（截断避免过长）
                let joined = stats.errors.join("; ");
                format!("检测到致命错误: {}", joined)
            };
            return Err(anyhow::anyhow!("构建代码索引失败：{}", reason));
        }

        tracing::info!(
            "索引构建完成: {}/{} 文件，{} 个向量，耗时 {:?}",
            processed_files - stats.failed_files.len(),
            total_files,
            stats.uploaded_vectors,
            stats.processing_time
        );

        Ok(stats)
    }

    /// 搜索代码向量
    pub async fn search_vectors(&self, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let start_time = Instant::now();

        tracing::info!("开始向量搜索: '{}'", options.query);

        // 1. 验证查询文本
        ensure!(options.query.len() >= 3, "查询文本过短，需要至少3个字符");
        ensure!(
            options.query.len() <= 1000,
            "查询文本过长，最多支持1000个字符"
        );

        // 2. 预处理查询文本
        let processed_query = self.preprocess_search_query(&options.query)?;

        // 3. 向量化查询（带重试机制）
        let query_vector = self
            .vectorize_with_retry(&processed_query)
            .await
            .context("查询向量化失败")?;

        // 4. 在Qdrant中搜索相似向量
        let results = self
            .storage
            .search_with_vector(query_vector, options.clone())
            .await
            .context("向量搜索失败")?;

        let search_duration = start_time.elapsed();

        tracing::info!(
            "搜索完成: 找到 {} 个结果，耗时 {:?}",
            results.len(),
            search_duration
        );

        // 5. 性能监控和警告
        if search_duration.as_millis() > 500 {
            tracing::warn!(
                "搜索耗时过长: {:?}，建议检查查询条件或索引性能",
                search_duration
            );
        }

        Ok(results)
    }

    /// 测试Qdrant连接
    pub async fn test_connection(&self) -> Result<String> {
        self.storage.test_connection().await
    }

    /// 获取集合信息
    pub async fn get_collection_info(&self) -> Result<(usize, usize)> {
        self.storage.get_collection_info().await
    }

    /// 获取集合名称
    pub fn collection_name(&self) -> &str {
        &self.config.user_config.collection_name
    }

    /// 获取向量化服务引用（供增量更新器使用）
    pub fn get_vectorizer(&self) -> &LLMVectorizationService {
        &self.vectorizer
    }

    /// 获取存储服务引用（供增量更新器使用）
    pub fn get_storage(&self) -> &QdrantClientImpl {
        &self.storage
    }

    /// 获取代码解析器引用（供增量更新器使用）
    pub fn get_parser(&self) -> &TreeSitterParser {
        &self.parser
    }

    /// 清空所有向量数据
    pub async fn clear_all_vectors(&self) -> Result<()> {
        self.storage.clear_all_vectors().await
    }

    /// 预处理搜索查询文本
    fn preprocess_search_query(&self, query: &str) -> Result<String> {
        // 1. 去除前后空格和小写化
        let normalized = query.trim().to_lowercase();

        // 2. 替换特殊字符为空格
        let cleaned = normalized
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() || c == '_' || c == '-' {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>();

        // 3. 合并多个空格并过滤短单词
        let processed = cleaned
            .split_whitespace()
            .filter(|word| word.len() >= 2) // 过滤过短的单词
            .collect::<Vec<_>>()
            .join(" ");

        ensure!(!processed.is_empty(), "预处理后的查询文本为空");

        tracing::debug!("查询预处理: '{}' -> '{}'", query, processed);
        Ok(processed)
    }

    /// 带重试机制的向量化
    async fn vectorize_with_retry(&self, text: &str) -> Result<Vec<f32>> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_DELAY: u64 = 100; // 毫秒

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.vectorizer.create_embedding(text).await {
                Ok(vector) => {
                    if attempt > 0 {
                        tracing::info!("向量化在第{}次尝试时成功", attempt + 1);
                    }
                    return Ok(vector);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < MAX_RETRIES - 1 {
                        let delay = INITIAL_DELAY * 2_u64.pow(attempt);
                        tracing::warn!(
                            "向量化失败（第{}/{}次尝试），{}ms后重试: {}",
                            attempt + 1,
                            MAX_RETRIES,
                            delay,
                            last_error.as_ref().unwrap()
                        );

                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        // 所有重试都失败，返回最后一个错误
        Err(last_error.unwrap())
    }
}
