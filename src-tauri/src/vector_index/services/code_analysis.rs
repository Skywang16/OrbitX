/*!
 * 代码分析服务
 *
 * 专注于代码解析和语义分块的服务。
 * 职责单一：将源代码文件转换为结构化的代码块。
 *
 * Requirements: 1.1, 1.2, 1.3
 */

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::vector_index::parser::{CodeParser, TreeSitterParser};
use crate::vector_index::types::{CodeChunk, CodeVector, Language, VectorIndexConfig};

/// 代码分析结果
#[derive(Debug)]
pub struct CodeAnalysisResult {
    pub file_path: String,
    pub language: Language,
    pub chunks: Vec<CodeChunk>,
    pub chunk_count: usize,
    pub analysis_time: std::time::Duration,
}

/// 代码分析服务接口
pub trait CodeAnalysisService: Send + Sync {
    /// 分析单个文件
    async fn analyze_file(&self, file_path: &str) -> Result<CodeAnalysisResult>;

    /// 批量分析文件
    async fn analyze_files(&self, file_paths: &[String]) -> Result<Vec<CodeAnalysisResult>>;

    /// 检查文件是否支持分析
    fn supports_file(&self, file_path: &str) -> bool;

    /// 获取支持的语言列表
    fn supported_languages(&self) -> Vec<Language>;
}

/// 基于Tree-sitter的代码分析服务实现
pub struct TreeSitterCodeAnalysisService {
    parser: Arc<TreeSitterParser>,
}

impl TreeSitterCodeAnalysisService {
    /// 创建新的代码分析服务
    pub fn new(config: VectorIndexConfig) -> Result<Self> {
        let parser = Arc::new(TreeSitterParser::new(config)?);
        Ok(Self { parser })
    }

    /// 为代码块生成向量ID
    fn generate_chunk_id(&self, file_path: &str, chunk: &CodeChunk) -> String {
        // 使用文件路径、起始行和内容哈希生成确定性ID
        let content_hash = md5::compute(&chunk.content);
        format!(
            "{}:{}:{}:{:x}",
            file_path, chunk.start_line, chunk.end_line, content_hash
        )
    }

    /// 将代码块转换为向量对象（不包含向量数据）
    pub fn chunk_to_vector_template(
        &self,
        chunk: &CodeChunk,
        file_path: &str,
        language: &Language,
    ) -> CodeVector {
        let id = self.generate_chunk_id(file_path, chunk);
        let mut metadata = chunk.metadata.clone();
        metadata.insert("file_path".to_string(), file_path.to_string());
        metadata.insert("language".to_string(), language.as_str().to_string());

        CodeVector {
            id,
            file_path: file_path.to_string(),
            content: chunk.content.clone(),
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            language: language.as_str().to_string(),
            chunk_type: chunk.chunk_type.as_str().to_string(),
            vector: Vec::new(), // 向量数据由向量化服务填充
            metadata,
        }
    }
}

impl CodeAnalysisService for TreeSitterCodeAnalysisService {
    async fn analyze_file(&self, file_path: &str) -> Result<CodeAnalysisResult> {
        let start_time = std::time::Instant::now();

        tracing::debug!("开始分析文件: {}", file_path);

        let parsed_code = self
            .parser
            .parse_file(file_path)
            .await
            .with_context(|| format!("解析文件失败: {}", file_path))?;

        // 检测文件语言
        let language = Language::from_extension(
            std::path::Path::new(file_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or(""),
        )
        .unwrap_or(Language::TypeScript); // 默认语言

        let analysis_time = start_time.elapsed();

        let result = CodeAnalysisResult {
            file_path: parsed_code.file_path,
            language,
            chunk_count: parsed_code.chunks.len(),
            chunks: parsed_code.chunks,
            analysis_time,
        };

        tracing::debug!(
            "文件分析完成: {} -> {} 个代码块，耗时 {:?}",
            file_path,
            result.chunk_count,
            analysis_time
        );

        Ok(result)
    }

    async fn analyze_files(&self, file_paths: &[String]) -> Result<Vec<CodeAnalysisResult>> {
        let mut results = Vec::new();

        tracing::info!("开始批量分析 {} 个文件", file_paths.len());

        for file_path in file_paths {
            match self.analyze_file(file_path).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    tracing::error!("文件分析失败 {}: {}", file_path, e);
                    // 继续处理其他文件
                }
            }
        }

        tracing::info!(
            "批量分析完成: 成功 {}/{} 文件",
            results.len(),
            file_paths.len()
        );

        Ok(results)
    }

    fn supports_file(&self, file_path: &str) -> bool {
        self.parser.supports_file(file_path)
    }

    fn supported_languages(&self) -> Vec<Language> {
        self.parser.supported_languages()
    }
}
