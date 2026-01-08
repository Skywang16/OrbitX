use crate::vector_db::core::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Embedder: Send + Sync {
    fn id(&self) -> &str;
    fn dim(&self) -> usize;
    fn model_name(&self) -> &str;
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

/// 创建 embedder (唯一入口)
pub fn create_embedder(
    config: &crate::vector_db::core::RemoteEmbeddingConfig,
) -> Result<Arc<dyn Embedder>> {
    Ok(Arc::new(super::remote::RemoteEmbedder::new(
        config.provider_config.clone(),
        config.model_name.clone(),
        config.dimension,
    )?))
}
