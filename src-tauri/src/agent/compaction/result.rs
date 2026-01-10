use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionResult {
    pub phase: CompactionPhase,
    pub tokens_before: u32,
    pub tokens_after: u32,
    pub tokens_saved: u32,
    pub tools_compacted: usize,
    pub summary_created: bool,
    pub summary_message_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionPhase {
    Started,
    Pruned,
    Compacted,
    Skipped,
}
