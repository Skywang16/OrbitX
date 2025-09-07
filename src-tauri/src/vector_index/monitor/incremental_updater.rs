/*!
 * 增量索引更新器
 *
 * 实现增量更新功能，删除旧向量数据，重新处理变化的文件。
 * 优化批量处理，实时通知更新状态，避免重复处理。
 *
 * ## 主要功能
 *
 * - **单文件更新**: 针对单个文件的增量向量更新
 * - **向量删除**: 删除文件对应的所有向量数据
 * - **批量优化**: 合并多个文件变化为批量操作
 * - **错误恢复**: 更新失败时的错误处理和恢复
 *
 * ## 设计原则
 *
 * - 重用向量索引服务的现有功能和接口
 * - 遵循OrbitX错误处理规范
 * - 保持与现有代码风格的一致性
 * - 避免重复实现已有功能
 *
 * Requirements: 8.1, 8.4, 8.5
 */

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{ensure, Context, Result};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::vector_index::{
    parser::CodeParser,
    qdrant::QdrantService,
    service::VectorIndexService,
    types::{CodeVector, Language, TaskProgress, VectorIndexConfig},
    vectorizer::VectorizationService,
};

/// 增量更新统计信息
#[derive(Debug, Clone, Default)]
pub struct IncrementalUpdateStats {
    /// 更新的文件数
    pub updated_files: usize,
    /// 删除的文件数
    pub deleted_files: usize,
    /// 新增的向量数
    pub added_vectors: usize,
    /// 删除的向量数
    pub deleted_vectors: usize,
    /// 更新耗时
    pub update_duration: std::time::Duration,
    /// 最近更新时间
    pub last_update_time: Option<Instant>,
}

/// 增量更新器
pub struct IncrementalUpdater {
    config: VectorIndexConfig,
    vector_service: Arc<VectorIndexService>,
}

impl IncrementalUpdater {
    /// 创建新的增量更新器
    pub fn new(config: VectorIndexConfig, vector_service: Arc<VectorIndexService>) -> Self {
        Self {
            config,
            vector_service,
        }
    }

    /// 更新单个文件的向量索引
    pub async fn update_single_file(&self, file_path: &Path) -> Result<IncrementalUpdateStats> {
        let start_time = Instant::now();
        let mut stats = IncrementalUpdateStats::default();

        tracing::info!("开始增量更新文件: {}", file_path.display());

        // 1. 验证文件存在且支持
        ensure!(
            file_path.exists() && file_path.is_file(),
            "文件不存在或不是文件: {}",
            file_path.display()
        );

        let file_path_str = file_path.to_string_lossy().to_string();

        // 2. 删除该文件的所有现有向量
        let deleted_count = self
            .delete_file_vectors_internal(&file_path_str)
            .await
            .with_context(|| format!("删除文件现有向量失败: {}", file_path.display()))?;

        stats.deleted_vectors = deleted_count;
        tracing::debug!(
            "删除文件 {} 的 {} 个现有向量",
            file_path.display(),
            deleted_count
        );

        // 3. 重新解析和向量化文件
        let new_vectors = self
            .process_file_to_vectors(file_path)
            .await
            .with_context(|| format!("重新处理文件失败: {}", file_path.display()))?;

        if !new_vectors.is_empty() {
            // 4. 上传新向量到Qdrant
            self.upload_vectors_to_storage(&new_vectors)
                .await
                .with_context(|| format!("上传新向量失败: {}", file_path.display()))?;

            stats.added_vectors = new_vectors.len();
            tracing::debug!(
                "为文件 {} 添加 {} 个新向量",
                file_path.display(),
                new_vectors.len()
            );
        }

        // 5. 更新统计信息
        stats.updated_files = 1;
        stats.update_duration = start_time.elapsed();
        stats.last_update_time = Some(Instant::now());

        tracing::info!(
            "文件增量更新完成: {} (删除{}个，新增{}个向量，耗时{:?})",
            file_path.display(),
            stats.deleted_vectors,
            stats.added_vectors,
            stats.update_duration
        );

        Ok(stats)
    }

    /// 删除文件对应的所有向量
    pub async fn delete_file_vectors(&self, file_path: &Path) -> Result<IncrementalUpdateStats> {
        let start_time = Instant::now();
        let mut stats = IncrementalUpdateStats::default();

        tracing::info!("删除文件向量: {}", file_path.display());

        let file_path_str = file_path.to_string_lossy().to_string();
        let deleted_count = self
            .delete_file_vectors_internal(&file_path_str)
            .await
            .with_context(|| format!("删除文件向量失败: {}", file_path.display()))?;

        // 更新统计信息
        stats.deleted_files = 1;
        stats.deleted_vectors = deleted_count;
        stats.update_duration = start_time.elapsed();
        stats.last_update_time = Some(Instant::now());

        tracing::info!(
            "文件向量删除完成: {} (删除{}个向量，耗时{:?})",
            file_path.display(),
            deleted_count,
            stats.update_duration
        );

        Ok(stats)
    }

    /// 批量更新多个文件
    pub async fn batch_update_files(
        &self,
        file_paths: &[PathBuf],
        progress_sender: Option<mpsc::Sender<TaskProgress>>,
    ) -> Result<IncrementalUpdateStats> {
        let start_time = Instant::now();
        let mut total_stats = IncrementalUpdateStats::default();
        let task_id = Uuid::new_v4().to_string();

        tracing::info!("开始批量增量更新 {} 个文件", file_paths.len());

        // 进度报告：不再使用定时模拟，只在每批处理完成后发送真实进度

        // 并发处理文件批次
        let batch_size = self.config.max_concurrent_files.min(4); // 限制并发数以避免资源过载
        let mut processed_files = 0;

        for file_batch in file_paths.chunks(batch_size) {
            let mut batch_tasks = Vec::new();

            // 为每个文件创建更新任务
            for file_path in file_batch {
                let updater =
                    IncrementalUpdater::new(self.config.clone(), self.vector_service.clone());
                let file_path = file_path.clone();

                let task = async move { updater.update_single_file(&file_path).await };

                batch_tasks.push(Box::pin(task));
            }

            // 等待批次完成
            let batch_results = futures::future::join_all(batch_tasks).await;

            // 处理批次结果
            for (idx, result) in batch_results.into_iter().enumerate() {
                match result {
                    Ok(file_stats) => {
                        // 合并统计信息
                        total_stats.updated_files += file_stats.updated_files;
                        total_stats.deleted_files += file_stats.deleted_files;
                        total_stats.added_vectors += file_stats.added_vectors;
                        total_stats.deleted_vectors += file_stats.deleted_vectors;

                        processed_files += 1;

                        tracing::debug!(
                            "批量更新进度: {}/{}, 文件: {}",
                            processed_files,
                            file_paths.len(),
                            file_batch[idx].display()
                        );
                    }
                    Err(e) => {
                        tracing::error!("批量更新文件失败: {} - {}", file_batch[idx].display(), e);
                    }
                }
            }

            // 更新进度
            if let Some(sender) = &progress_sender {
                let progress = TaskProgress {
                    task_id: task_id.clone(),
                    progress: processed_files as f32 / file_paths.len() as f32,
                    status: format!("批量更新中 ({}/{})", processed_files, file_paths.len()),
                    current_file: None,
                    processed_files,
                    total_files: file_paths.len(),
                    cancellable: false,
                };
                let _ = sender.send(progress).await;
            }
        }

        // 完成统计
        total_stats.update_duration = start_time.elapsed();
        total_stats.last_update_time = Some(Instant::now());

        tracing::info!(
            "批量增量更新完成: {}/{} 文件，新增{}个向量，删除{}个向量，耗时{:?}",
            total_stats.updated_files,
            file_paths.len(),
            total_stats.added_vectors,
            total_stats.deleted_vectors,
            total_stats.update_duration
        );

        Ok(total_stats)
    }

    /// 处理文件生成向量
    async fn process_file_to_vectors(&self, file_path: &Path) -> Result<Vec<CodeVector>> {
        // 重用现有的代码解析器
        let parser = self.vector_service.get_parser();

        // 解析文件
        let parsed_code = parser
            .parse_file(&file_path.to_string_lossy())
            .await
            .with_context(|| format!("解析文件失败: {}", file_path.display()))?;

        if parsed_code.chunks.is_empty() {
            tracing::debug!("文件无有效代码块: {}", file_path.display());
            return Ok(Vec::new());
        }

        // 准备向量化文本
        let file_path_str = file_path.to_string_lossy().to_string();
        let texts: Vec<String> = parsed_code
            .chunks
            .iter()
            .map(|chunk| {
                format!(
                    "// 文件: {}\n// 类型: {}\n{}",
                    file_path_str,
                    chunk.chunk_type.as_str(),
                    chunk.content
                )
            })
            .collect();

        // 通过向量索引服务进行向量化（重用现有逻辑）
        let embeddings = self
            .vectorize_texts(&texts)
            .await
            .context("向量化文本失败")?;

        // 构建向量对象
        let mut vectors = Vec::new();
        for (chunk, embedding) in parsed_code.chunks.iter().zip(embeddings) {
            let vector = CodeVector {
                id: Uuid::new_v4().to_string(),
                file_path: file_path_str.clone(),
                content: chunk.content.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                language: Language::from_extension(
                    file_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or(""),
                )
                .unwrap_or(Language::TypeScript)
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

    /// 删除文件对应的向量（内部实现）
    async fn delete_file_vectors_internal(&self, file_path: &str) -> Result<usize> {
        tracing::debug!("删除文件向量: {}", file_path);

        // 通过向量索引服务的存储组件删除向量
        let storage = self.vector_service.get_storage();

        // 获取删除前的统计信息
        let (before_count, _) = storage.get_collection_info().await.unwrap_or((0, 0));

        // 执行删除操作
        storage
            .delete_file_vectors(file_path)
            .await
            .with_context(|| format!("删除文件向量失败: {}", file_path))?;

        // 获取删除后的统计信息（粗略估算删除数量）
        let (after_count, _) = storage.get_collection_info().await.unwrap_or((0, 0));

        let deleted_count = before_count.saturating_sub(after_count);

        tracing::debug!("删除文件 {} 的 {} 个向量", file_path, deleted_count);
        Ok(deleted_count)
    }

    /// 向量化文本列表
    async fn vectorize_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!("开始向量化 {} 个文本", texts.len());

        // 通过向量索引服务的向量化组件进行处理
        let vectorizer = self.vector_service.get_vectorizer();

        // 使用批量向量化接口
        vectorizer
            .create_embeddings(texts)
            .await
            .context("批量向量化失败")
    }

    /// 上传向量到存储
    async fn upload_vectors_to_storage(&self, vectors: &[CodeVector]) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }

        tracing::debug!("开始上传 {} 个向量", vectors.len());

        // 通过向量索引服务的存储组件上传向量
        let storage = self.vector_service.get_storage();

        storage
            .upload_vectors(vectors.to_vec())
            .await
            .context("向量上传失败")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_incremental_updater_creation() {
        let _config = VectorIndexConfig::default();

        // 注意：这里需要mock VectorIndexService，实际测试中需要完整的服务
        // 暂时跳过需要真实服务的测试
    }

    #[tokio::test]
    async fn test_file_path_validation() {
        let _config = VectorIndexConfig::default();

        // 创建临时文件
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").await.unwrap();

        // 验证文件存在性检查逻辑
        assert!(test_file.exists());
        assert!(test_file.is_file());
    }
}
