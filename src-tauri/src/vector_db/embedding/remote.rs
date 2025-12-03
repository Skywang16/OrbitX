use async_trait::async_trait;
use crate::vector_db::core::{Result, VectorDbError};
use super::Embedder;
use crate::llm::{
    types::{EmbeddingRequest, LLMProviderConfig},
    providers::base::LLMProvider,
    providers::openai::OpenAIProvider,
};

pub struct RemoteEmbedder {
    provider: OpenAIProvider,
    model_name: String,
    dim: usize,
}

impl RemoteEmbedder {
    pub fn new(config: LLMProviderConfig, model_name: String, dim: usize) -> Result<Self> {
        Ok(Self {
            provider: OpenAIProvider::new(config),
            model_name,
            dim,
        })
    }
}

#[async_trait]
impl Embedder for RemoteEmbedder {
    fn id(&self) -> &str {
        "remote"
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            model: self.model_name.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
            encoding_format: None,
            dimensions: Some(self.dim),
        };

        self.provider
            .create_embeddings(request)
            .await
            .map(|resp| resp.data.into_iter().map(|d| d.embedding).collect())
            .map_err(|e| VectorDbError::Embedding(e.to_string()))
    }
}
