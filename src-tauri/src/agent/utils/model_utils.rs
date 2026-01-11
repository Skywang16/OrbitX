use crate::storage::repositories::AIModels;
use crate::storage::DatabaseManager;
use std::sync::Arc;

/// 获取模型的上下文窗口大小
pub async fn get_model_context_window(
    database: &Arc<DatabaseManager>,
    model_id: &str,
) -> Option<u32> {
    let model = AIModels::new(database).find_by_id(model_id).await.ok()??;

    model
        .options
        .and_then(|opts| opts.get("maxContextTokens")?.as_u64())
        .map(|v| v as u32)
}
