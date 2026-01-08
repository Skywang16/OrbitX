use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: FileChangeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub message: String,
    #[serde(default)]
    pub refs: Vec<CommitRef>,
    #[serde(default)]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitRef {
    pub name: String,
    pub ref_type: CommitRefType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommitRefType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Head,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryStatus {
    pub is_repository: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    pub staged_files: Vec<FileChange>,
    pub modified_files: Vec<FileChange>,
    pub untracked_files: Vec<FileChange>,
    pub conflicted_files: Vec<FileChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
    pub is_empty: bool,
    pub is_detached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffContent {
    pub file_path: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub code: GitErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub message: String,
    pub files: Vec<CommitFileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileChange {
    pub path: String,
    pub status: FileChangeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GitErrorCode {
    NotARepository,
    GitNotInstalled,
    CommandFailed,
    ParseError,
    IoError,
}

