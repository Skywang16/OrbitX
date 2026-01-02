use crate::git::types::*;
use std::io;
use tokio::process::Command as AsyncCommand;

pub struct GitService;

impl GitService {
    async fn ensure_repo_root(path: &str) -> Result<String, GitError> {
        match Self::is_repository(path).await? {
            Some(root) => Ok(root),
            None => Err(GitError {
                code: GitErrorCode::NotARepository,
                message: "git.not_a_repository".to_string(),
            }),
        }
    }

    async fn execute(args: &[&str], cwd: &str) -> Result<Vec<u8>, GitError> {
        let mut cmd = AsyncCommand::new("git");
        cmd.args(args);
        if !cwd.trim().is_empty() {
            cmd.current_dir(cwd);
        }

        let output = cmd.output().await.map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => GitError {
                code: GitErrorCode::GitNotInstalled,
                message: "git.not_installed".to_string(),
            },
            _ => GitError {
                code: GitErrorCode::IoError,
                message: e.to_string(),
            },
        })?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError {
                code: GitErrorCode::CommandFailed,
                message: if stderr.is_empty() {
                    "git.command_failed".to_string()
                } else {
                    stderr
                },
            })
        }
    }

    async fn execute_text(args: &[&str], cwd: &str) -> Result<String, GitError> {
        let bytes = Self::execute(args, cwd).await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn is_not_a_repository(stderr_or_message: &str) -> bool {
        let msg = stderr_or_message.to_lowercase();
        msg.contains("not a git repository")
            || msg.contains("fatal: not a git repository")
            || msg.contains("fatal: not a repository")
            || msg.contains("fatal:") && msg.contains("repository")
    }

    pub async fn is_repository(path: &str) -> Result<Option<String>, GitError> {
        match Self::execute_text(&["rev-parse", "--show-toplevel"], path).await {
            Ok(text) => Ok(Some(text.trim().to_string())),
            Err(e) if e.code == GitErrorCode::CommandFailed && Self::is_not_a_repository(&e.message) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_status(path: &str) -> Result<RepositoryStatus, GitError> {
        let root = match Self::is_repository(path).await? {
            Some(root) => root,
            None => {
                return Ok(RepositoryStatus {
                    is_repository: false,
                    root_path: None,
                    current_branch: None,
                    staged_files: vec![],
                    modified_files: vec![],
                    untracked_files: vec![],
                    conflicted_files: vec![],
                    ahead: None,
                    behind: None,
                    is_empty: false,
                    is_detached: false,
                })
            }
        };

        let output = match Self::execute(&["status", "--porcelain=v1", "--branch", "-z"], &root).await {
            Ok(bytes) => bytes,
            Err(e) if e.code == GitErrorCode::CommandFailed && Self::is_not_a_repository(&e.message) => {
                return Ok(RepositoryStatus {
                    is_repository: false,
                    root_path: None,
                    current_branch: None,
                    staged_files: vec![],
                    modified_files: vec![],
                    untracked_files: vec![],
                    conflicted_files: vec![],
                    ahead: None,
                    behind: None,
                    is_empty: false,
                    is_detached: false,
                })
            }
            Err(e) => return Err(e),
        };

        let parsed = Self::parse_status_porcelain_v1_z(&output).ok_or(GitError {
            code: GitErrorCode::ParseError,
            message: "git.parse_error".to_string(),
        })?;

        Ok(RepositoryStatus {
            is_repository: true,
            root_path: Some(root),
            current_branch: parsed.current_branch,
            staged_files: parsed.staged_files,
            modified_files: parsed.modified_files,
            untracked_files: parsed.untracked_files,
            conflicted_files: parsed.conflicted_files,
            ahead: parsed.ahead,
            behind: parsed.behind,
            is_empty: parsed.is_empty,
            is_detached: parsed.is_detached,
        })
    }

    pub async fn get_branches(path: &str) -> Result<Vec<BranchInfo>, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        let locals = Self::execute_text(
            &[
                "for-each-ref",
                "refs/heads",
                "--format=%(refname:short)\t%(HEAD)\t%(upstream:short)",
            ],
            &root,
        )
        .await?;

        let remotes = Self::execute_text(
            &["for-each-ref", "refs/remotes", "--format=%(refname:short)"],
            &root,
        )
        .await?;

        let mut branches: Vec<BranchInfo> = Vec::new();
        let mut current_local: Option<(String, Option<String>)> = None;

        for line in locals.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let mut parts = line.split('\t');
            let name = parts.next().unwrap_or_default().trim().to_string();
            if name.is_empty() {
                continue;
            }
            let head = parts.next().unwrap_or_default().trim();
            let upstream = parts.next().unwrap_or_default().trim().to_string();

            let is_current = head == "*";
            let upstream_opt = if upstream.is_empty() { None } else { Some(upstream) };

            if is_current {
                current_local = Some((name.clone(), upstream_opt.clone()));
            }

            branches.push(BranchInfo {
                name,
                is_current,
                is_remote: false,
                upstream: upstream_opt,
                ahead: None,
                behind: None,
            });
        }

        for line in remotes.lines() {
            let name = line.trim().to_string();
            if name.is_empty() || name.ends_with("/HEAD") {
                continue;
            }
            branches.push(BranchInfo {
                name,
                is_current: false,
                is_remote: true,
                upstream: None,
                ahead: None,
                behind: None,
            });
        }

        if let Some((branch, Some(upstream))) = current_local {
            if let Ok((ahead, behind)) = Self::get_ahead_behind(&root, &branch, &upstream).await {
                if let Some(current) = branches.iter_mut().find(|b| b.is_current && !b.is_remote) {
                    current.ahead = Some(ahead);
                    current.behind = Some(behind);
                }
            }
        }

        Ok(branches)
    }

    pub async fn get_commits(path: &str, limit: u32) -> Result<Vec<CommitInfo>, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        let limit = limit.max(1).min(200);
        // Use -z for NUL-separated records, %x1f for field separator, %s for subject (no newlines)
        let format = "%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%D%x1f%P%x1f%s";
        let n_arg = format!("-n{}", limit);
        let pretty_arg = format!("--pretty=format:{}", format);
        let args = ["log", "-z", n_arg.as_str(), "--date=iso-strict", pretty_arg.as_str()];

        let output = match Self::execute(&args, &root).await {
            Ok(bytes) => bytes,
            Err(e) if e.code == GitErrorCode::CommandFailed => {
                let msg = e.message.to_lowercase();
                if msg.contains("does not have any commits yet")
                    || msg.contains("your current branch")
                        && msg.contains("does not have any commits yet")
                    || msg.contains("unknown revision or path not in the working tree")
                {
                    return Ok(vec![]);
                }
                return Err(e);
            }
            Err(e) => return Err(e),
        };

        Ok(Self::parse_commits(&output))
    }

    pub async fn get_diff(path: &str, file_path: &str, staged: bool) -> Result<DiffContent, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        let mut args = vec!["diff", "--no-color", "--unified=3"];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(file_path);

        let output = Self::execute(&args, &root).await?;
        Ok(Self::parse_unified_diff(file_path, &output))
    }

    pub async fn get_commit_file_diff(
        path: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<DiffContent, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        // Get diff for a specific file in a commit
        // Use commit^..commit to show diff between parent and commit
        let range = format!("{}^..{}", commit_hash, commit_hash);
        let args = ["diff", "--no-color", "--unified=3", &range, "--", file_path];

        let output = match Self::execute(&args, &root).await {
            Ok(bytes) => bytes,
            Err(e) if e.code == GitErrorCode::CommandFailed => {
                // If commit has no parent (initial commit), diff against empty tree
                let args = ["show", "--no-color", "--unified=3", "--format=", commit_hash, "--", file_path];
                Self::execute(&args, &root).await?
            }
            Err(e) => return Err(e),
        };

        Ok(Self::parse_unified_diff(file_path, &output))
    }

    pub async fn get_commit_files(path: &str, commit_hash: &str) -> Result<Vec<CommitFileChange>, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        // Use -m to show files for merge commits, --first-parent to show diff against first parent only
        let output = Self::execute_text(
            &["show", "-m", "--first-parent", "--numstat", "--format=", "--no-color", commit_hash],
            &root,
        )
        .await?;

        let status_output = Self::execute_text(
            &["show", "-m", "--first-parent", "--name-status", "--format=", "--no-color", commit_hash],
            &root,
        )
        .await?;

        let mut files: Vec<CommitFileChange> = Vec::new();

        // Parse numstat for additions/deletions
        let mut numstat_map: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let additions: u32 = parts[0].parse().unwrap_or(0);
                let deletions: u32 = parts[1].parse().unwrap_or(0);
                let file_path = parts[2].to_string();
                numstat_map.insert(file_path, (additions, deletions));
            }
        }

        // Parse name-status for file status
        for line in status_output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() {
                continue;
            }

            let status_char = parts[0].chars().next().unwrap_or(' ');
            let (status, file_path, old_path) = match status_char {
                'A' => {
                    let path = parts.get(1).unwrap_or(&"").to_string();
                    (FileChangeStatus::Added, path, None)
                }
                'M' => {
                    let path = parts.get(1).unwrap_or(&"").to_string();
                    (FileChangeStatus::Modified, path, None)
                }
                'D' => {
                    let path = parts.get(1).unwrap_or(&"").to_string();
                    (FileChangeStatus::Deleted, path, None)
                }
                'R' => {
                    let old = parts.get(1).unwrap_or(&"").to_string();
                    let new = parts.get(2).unwrap_or(&"").to_string();
                    (FileChangeStatus::Renamed, new, Some(old))
                }
                'C' => {
                    let old = parts.get(1).unwrap_or(&"").to_string();
                    let new = parts.get(2).unwrap_or(&"").to_string();
                    (FileChangeStatus::Copied, new, Some(old))
                }
                _ => continue,
            };

            let (additions, deletions) = numstat_map.get(&file_path).copied().unwrap_or((0, 0));

            files.push(CommitFileChange {
                path: file_path,
                status,
                old_path,
                additions,
                deletions,
            });
        }

        Ok(files)
    }

    async fn get_ahead_behind(cwd: &str, branch: &str, upstream: &str) -> Result<(u32, u32), GitError> {
        let range = format!("{}...{}", branch, upstream);
        let output = Self::execute_text(&["rev-list", "--left-right", "--count", &range], cwd).await?;
        let mut parts = output.split_whitespace();
        let ahead: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let behind: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        Ok((ahead, behind))
    }

    fn parse_status_porcelain_v1_z(bytes: &[u8]) -> Option<ParsedStatus> {
        let entries: Vec<&[u8]> = bytes.split(|b| *b == 0).filter(|s| !s.is_empty()).collect();
        if entries.is_empty() {
            return Some(ParsedStatus::default());
        }

        let mut parsed = ParsedStatus::default();

        // branch info is usually the first record, but don't rely on it.
        let mut i = 0;
        while i < entries.len() {
            let entry = entries[i];
            if entry.starts_with(b"## ") {
                let line = String::from_utf8_lossy(&entry[3..]).trim().to_string();
                Self::parse_branch_summary(&line, &mut parsed);
                i += 1;
                continue;
            }

            if entry.len() < 3 {
                i += 1;
                continue;
            }

            let x = entry[0] as char;
            let y = entry[1] as char;

            let path_part = String::from_utf8_lossy(&entry[2..]).trim().to_string();

            if x == '?' && y == '?' {
                parsed.untracked_files.push(FileChange {
                    path: path_part,
                    status: FileChangeStatus::Untracked,
                    old_path: None,
                });
                i += 1;
                continue;
            }

            let is_unmerged = matches!(
                (x, y),
                ('D', 'D') | ('A', 'U') | ('U', 'D') | ('U', 'A') | ('D', 'U') | ('A', 'A') | ('U', 'U')
            );

            if is_unmerged || x == 'U' || y == 'U' {
                parsed.conflicted_files.push(FileChange {
                    path: path_part,
                    status: FileChangeStatus::Conflicted,
                    old_path: None,
                });
                i += 1;
                continue;
            }

            let mut old_path: Option<String> = None;
            let mut new_path = path_part.clone();
            let is_rename_or_copy = x == 'R' || x == 'C' || y == 'R' || y == 'C';
            if is_rename_or_copy {
                old_path = Some(path_part);
                if i + 1 < entries.len() {
                    new_path = String::from_utf8_lossy(entries[i + 1]).to_string();
                    i += 1;
                }
            }

            if x != ' ' {
                if let Some(status) = Self::map_status_char(x, true) {
                    parsed.staged_files.push(FileChange {
                        path: new_path.clone(),
                        status,
                        old_path: old_path.clone(),
                    });
                }
            }

            if y != ' ' {
                if let Some(status) = Self::map_status_char(y, false) {
                    parsed.modified_files.push(FileChange {
                        path: new_path.clone(),
                        status,
                        old_path,
                    });
                }
            }

            i += 1;
        }

        Some(parsed)
    }

    fn map_status_char(ch: char, is_index: bool) -> Option<FileChangeStatus> {
        match ch {
            'A' => Some(FileChangeStatus::Added),
            'M' => Some(FileChangeStatus::Modified),
            'D' => Some(FileChangeStatus::Deleted),
            'R' => Some(FileChangeStatus::Renamed),
            'C' => Some(FileChangeStatus::Copied),
            '?' => Some(FileChangeStatus::Untracked),
            '!' => None,
            ' ' => None,
            _ => {
                // Treat unknown worktree status as Modified to avoid dropping changes.
                if !is_index {
                    Some(FileChangeStatus::Modified)
                } else {
                    None
                }
            }
        }
    }

    fn parse_branch_summary(summary: &str, parsed: &mut ParsedStatus) {
        let summary = summary.trim();
        if summary.is_empty() {
            return;
        }

        if summary == "HEAD (no branch)" {
            parsed.is_detached = true;
            parsed.current_branch = None;
            return;
        }

        if let Some(rest) = summary.strip_prefix("No commits yet on ") {
            parsed.is_empty = true;
            parsed.current_branch = Some(rest.trim().to_string());
            return;
        }

        if let Some(rest) = summary.strip_prefix("Initial commit on ") {
            parsed.is_empty = true;
            parsed.current_branch = Some(rest.trim().to_string());
            return;
        }

        // Format: <branch>...<upstream> [ahead N, behind M]
        // Or: <branch> [ahead N]
        let (head_part, bracket_part) = match summary.split_once(" [") {
            Some((left, right)) => (left.trim(), Some(right.trim_end_matches(']').trim())),
            None => (summary, None),
        };

        let (branch, _upstream) = match head_part.split_once("...") {
            Some((b, u)) => (b.trim(), Some(u.trim())),
            None => (head_part.trim(), None),
        };

        parsed.current_branch = if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        };

        if let Some(bracket) = bracket_part {
            for part in bracket.split(',') {
                let p = part.trim();
                if let Some(num) = p.strip_prefix("ahead ") {
                    parsed.ahead = num.trim().parse::<u32>().ok();
                } else if let Some(num) = p.strip_prefix("behind ") {
                    parsed.behind = num.trim().parse::<u32>().ok();
                }
            }
        }
    }

    fn parse_commits(output: &[u8]) -> Vec<CommitInfo> {
        let mut commits = Vec::new();
        // Split by NUL (0x00), which -z flag uses as record separator
        for record in output.split(|b| *b == 0) {
            if record.is_empty() {
                continue;
            }

            // Split by Unit Separator (0x1f) for fields
            let fields: Vec<&[u8]> = record.split(|b| *b == 0x1f).collect();
            if fields.len() < 8 {
                continue;
            }

            let refs_str = String::from_utf8_lossy(fields[5]).to_string();
            let refs = Self::parse_refs(&refs_str);

            let parents_str = String::from_utf8_lossy(fields[6]).to_string();
            let parents: Vec<String> = parents_str
                .split_whitespace()
                .map(|s| s[..7.min(s.len())].to_string())
                .collect();

            commits.push(CommitInfo {
                hash: String::from_utf8_lossy(fields[0]).to_string(),
                short_hash: String::from_utf8_lossy(fields[1]).to_string(),
                author_name: String::from_utf8_lossy(fields[2]).to_string(),
                author_email: String::from_utf8_lossy(fields[3]).to_string(),
                date: String::from_utf8_lossy(fields[4]).to_string(),
                message: String::from_utf8_lossy(fields[7]).to_string(),
                refs,
                parents,
            });
        }
        commits
    }

    fn parse_refs(refs_str: &str) -> Vec<CommitRef> {
        let mut refs = Vec::new();
        if refs_str.trim().is_empty() {
            return refs;
        }

        for part in refs_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // HEAD -> branch_name
            if part.starts_with("HEAD -> ") {
                if let Some(branch) = part.strip_prefix("HEAD -> ") {
                    refs.push(CommitRef {
                        name: "HEAD".to_string(),
                        ref_type: CommitRefType::Head,
                    });
                    refs.push(CommitRef {
                        name: branch.to_string(),
                        ref_type: CommitRefType::LocalBranch,
                    });
                }
                continue;
            }

            // HEAD only
            if part == "HEAD" {
                refs.push(CommitRef {
                    name: "HEAD".to_string(),
                    ref_type: CommitRefType::Head,
                });
                continue;
            }

            // tag: tag_name
            if let Some(tag) = part.strip_prefix("tag: ") {
                refs.push(CommitRef {
                    name: tag.to_string(),
                    ref_type: CommitRefType::Tag,
                });
                continue;
            }

            // origin/branch_name (remote branch)
            if part.contains('/') {
                refs.push(CommitRef {
                    name: part.to_string(),
                    ref_type: CommitRefType::RemoteBranch,
                });
                continue;
            }

            // local branch
            refs.push(CommitRef {
                name: part.to_string(),
                ref_type: CommitRefType::LocalBranch,
            });
        }

        refs
    }

    fn parse_unified_diff(file_path: &str, output: &[u8]) -> DiffContent {
        let text = String::from_utf8_lossy(output);
        let mut hunks: Vec<DiffHunk> = Vec::new();

        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line: Option<u32> = None;
        let mut new_line: Option<u32> = None;

        for raw_line in text.lines() {
            if raw_line.starts_with("@@") {
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }
                let (o, n) = Self::parse_hunk_header(raw_line);
                old_line = o;
                new_line = n;
                current_hunk = Some(DiffHunk {
                    header: raw_line.to_string(),
                    lines: Vec::new(),
                });
                continue;
            }

            let Some(hunk) = current_hunk.as_mut() else {
                continue;
            };

            if raw_line.starts_with("diff ")
                || raw_line.starts_with("index ")
                || raw_line.starts_with("--- ")
                || raw_line.starts_with("+++ ")
            {
                hunk.lines.push(DiffLine {
                    line_type: DiffLineType::Header,
                    content: raw_line.to_string(),
                    old_line_number: None,
                    new_line_number: None,
                });
                continue;
            }

            let (line_type, next_old, next_new) = match raw_line.chars().next() {
                Some('+') => (
                    DiffLineType::Added,
                    old_line,
                    new_line.map(|n| n + 1),
                ),
                Some('-') => (
                    DiffLineType::Removed,
                    old_line.map(|o| o + 1),
                    new_line,
                ),
                Some(' ') => (
                    DiffLineType::Context,
                    old_line.map(|o| o + 1),
                    new_line.map(|n| n + 1),
                ),
                Some('\\') => (DiffLineType::Header, old_line, new_line),
                _ => (DiffLineType::Header, old_line, new_line),
            };

            let old_num = match line_type {
                DiffLineType::Removed | DiffLineType::Context => old_line,
                _ => None,
            };
            let new_num = match line_type {
                DiffLineType::Added | DiffLineType::Context => new_line,
                _ => None,
            };

            hunk.lines.push(DiffLine {
                line_type,
                content: raw_line.to_string(),
                old_line_number: old_num,
                new_line_number: new_num,
            });

            old_line = next_old;
            new_line = next_new;
        }

        if let Some(hunk) = current_hunk.take() {
            hunks.push(hunk);
        }

        DiffContent {
            file_path: file_path.to_string(),
            hunks,
        }
    }

    fn parse_hunk_header(header: &str) -> (Option<u32>, Option<u32>) {
        // @@ -old_start,old_len +new_start,new_len @@
        let mut old_start: Option<u32> = None;
        let mut new_start: Option<u32> = None;

        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 3 {
            return (None, None);
        }

        if let Some(old_part) = parts[1].strip_prefix('-') {
            old_start = old_part
                .split(',')
                .next()
                .and_then(|s| s.parse::<u32>().ok());
        }

        if let Some(new_part) = parts[2].strip_prefix('+') {
            new_start = new_part
                .split(',')
                .next()
                .and_then(|s| s.parse::<u32>().ok());
        }

        (old_start, new_start)
    }
}

#[derive(Default)]
struct ParsedStatus {
    current_branch: Option<String>,
    staged_files: Vec<FileChange>,
    modified_files: Vec<FileChange>,
    untracked_files: Vec<FileChange>,
    conflicted_files: Vec<FileChange>,
    ahead: Option<u32>,
    behind: Option<u32>,
    is_empty: bool,
    is_detached: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_types_serialize_roundtrip() {
        let status = RepositoryStatus {
            is_repository: true,
            root_path: Some("/tmp/repo".to_string()),
            current_branch: Some("main".to_string()),
            staged_files: vec![FileChange {
                path: "a.txt".to_string(),
                status: FileChangeStatus::Added,
                old_path: None,
            }],
            modified_files: vec![],
            untracked_files: vec![],
            conflicted_files: vec![],
            ahead: Some(1),
            behind: Some(2),
            is_empty: false,
            is_detached: false,
        };

        let json = serde_json::to_string(&status).unwrap();
        let back: RepositoryStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }

    #[test]
    fn parse_branch_summary_detached() {
        let mut parsed = ParsedStatus::default();
        GitService::parse_branch_summary("HEAD (no branch)", &mut parsed);
        assert!(parsed.is_detached);
        assert!(parsed.current_branch.is_none());
    }

    #[test]
    fn parse_branch_summary_ahead_behind() {
        let mut parsed = ParsedStatus::default();
        GitService::parse_branch_summary("main...origin/main [ahead 3, behind 1]", &mut parsed);
        assert_eq!(parsed.current_branch.as_deref(), Some("main"));
        assert_eq!(parsed.ahead, Some(3));
        assert_eq!(parsed.behind, Some(1));
    }

    #[test]
    fn parse_status_porcelain_v1_z_basic() {
        let raw = b"## main...origin/main [ahead 1]\0 M file.txt\0?? new.txt\0";
        let parsed = GitService::parse_status_porcelain_v1_z(raw).unwrap();
        assert_eq!(parsed.current_branch.as_deref(), Some("main"));
        assert_eq!(parsed.ahead, Some(1));
        assert_eq!(parsed.modified_files.len(), 1);
        assert_eq!(parsed.untracked_files.len(), 1);
        assert_eq!(parsed.modified_files[0].path, "file.txt");
        assert_eq!(parsed.untracked_files[0].path, "new.txt");
    }
}
