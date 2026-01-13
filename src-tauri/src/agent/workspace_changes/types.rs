use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone)]
pub struct ObservedChange {
    pub abs_path: String,
    pub old_abs_path: Option<String>,
    pub kind: ChangeKind,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingChange {
    pub relative_path: String,
    pub kind: ChangeKind,
    pub observed_at_ms: u64,
    pub patch: Option<String>,
    pub large_change: bool,
    pub note: Option<String>,
}
