/*!
 * 向量化服务模块
 *
 * 专注于将代码文本转换为向量表示的核心服务。
 * 基于OrbitX现有的LLM服务架构，提供统一的embedding生成功能。
 *
 * ## 主要功能
 *
 * - **文本向量化**: 将代码文本转换为高维向量表示
 * - **批量处理**: 支持大规模文本的批量向量化
 * - **错误处理**: 完善的重试机制和错误恢复
 * - **LLM集成**: 重用现有LLMService接口和配置
 *
 * ## 设计特点
 *
 * - 职责单一：专注文本到向量的转换
 * - 不包含批量处理逻辑，保持接口简洁
 * - 支持多种embedding模型和提供商
 *
 * Requirements: 2.1, 2.2, 2.4
 */

use std::sync::Arc;

use anyhow::Result;

use crate::llm::service::LLMService;
use crate::llm::types::EmbeddingRequest;

/// 向量化服务接口 - 专注文本到向量的转换
pub trait VectorizationService: Send + Sync {
    /// 为单个文本生成embedding
    fn create_embedding(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<Vec<f32>>> + Send;

    /// 批量生成embedding
    fn create_embeddings(
        &self,
        texts: &[String],
    ) -> impl std::future::Future<Output = Result<Vec<Vec<f32>>>> + Send;
}

/// 基于LLM的向量化服务实现
pub struct LLMVectorizationService {
    llm_service: Arc<LLMService>,
    embedding_model: String,
    max_retries: usize,
}

impl LLMVectorizationService {
    /// 创建新的向量化服务
    pub fn new(llm_service: Arc<LLMService>, embedding_model: String) -> Self {
        Self {
            llm_service,
            embedding_model,
            max_retries: 3, // 默认重试次数
        }
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 检测是否为不可恢复的致命错误（不应重试）
    fn is_fatal_error(&self, err: &anyhow::Error) -> bool {
        let msg = err.to_string().to_lowercase();
        let keywords = [
            "model is not embedding",
            "model not found",
            "模型未找到",
            "解密",
            "aead::error",
        ];
        keywords.iter().any(|k| msg.contains(k))
    }

    /// 为单个文本生成embedding（带重试机制）
    async fn create_embedding_with_retry(&self, text: &str) -> Result<Vec<f32>> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tracing::warn!("Embedding API重试第{}次", attempt);
                tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt as u64)).await;
            }

            let request = EmbeddingRequest {
                model: self.embedding_model.clone(),
                input: vec![text.to_string()],
                encoding_format: Some("float".to_string()),
                dimensions: None,
            };

            match self.llm_service.create_embeddings(request).await {
                Ok(response) => {
                    if let Some(data) = response.data.first() {
                        return Ok(data.embedding.clone());
                    } else {
                        last_error = Some(anyhow::anyhow!("embedding响应为空"));
                    }
                }
                Err(e) => {
                    // 对不可恢复错误直接返回，避免无意义重试
                    if self.is_fatal_error(&e) {
                        tracing::error!(
                            "Embedding API不可恢复错误（不重试）：{}",
                            e
                        );
                        return Err(e);
                    }
                    tracing::warn!("Embedding API调用失败 (尝试{}): {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("embedding调用失败")))
    }
}

impl VectorizationService for LLMVectorizationService {
    async fn create_embedding(&self, text: &str) -> Result<Vec<f32>> {
        self.create_embedding_with_retry(text).await
    }

    async fn create_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: texts.to_vec(),
            encoding_format: Some("float".to_string()),
            dimensions: None,
        };

        match self.llm_service.create_embeddings(request).await {
            Ok(response) => {
                let mut results = Vec::new();
                for data in response.data {
                    results.push(data.embedding);
                }
                Ok(results)
            }
            Err(e) => {
                // 若为不可恢复错误，直接返回错误，避免无意义回退
                let fatal = {
                    let msg = e.to_string().to_lowercase();
                    msg.contains("model is not embedding")
                        || msg.contains("model not found")
                        || msg.contains("模型未找到")
                        || msg.contains("解密")
                        || msg.contains("aead::error")
                };
                if fatal {
                    tracing::error!("批量embedding发生不可恢复错误：{}", e);
                    return Err(e);
                }
                tracing::warn!("批量embedding失败，回退到单个处理: {}", e);
                // 回退到单个处理
                let mut results = Vec::new();
                for text in texts {
                    match self.create_embedding(text).await {
                        Ok(embedding) => results.push(embedding),
                        Err(err) => {
                            tracing::error!("单个embedding失败: {}", err);
                            return Err(err);
                        }
                    }
                }
                Ok(results)
            }
        }
    }
}
