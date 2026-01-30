use crate::git::types::*;
use std::io;
use tokio::process::Command as AsyncCommand;

pub struct GitService;

impl GitService {
    fn not_a_repo_status() -> RepositoryStatus {
        RepositoryStatus {
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
        }
    }

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

    async fn execute_no_output(args: &[&str], cwd: &str) -> Result<(), GitError> {
        let _ = Self::execute(args, cwd).await?;
        Ok(())
    }

    async fn execute_with_paths(
        args_prefix: &[&str],
        paths: &[String],
        cwd: &str,
    ) -> Result<(), GitError> {
        let mut cmd = AsyncCommand::new("git");
        cmd.args(args_prefix);
        cmd.args(paths);
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
            Ok(())
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

    /// 执行命令，如果不是 git 仓库则返回 Ok(None)
    async fn execute_optional(args: &[&str], cwd: &str) -> Result<Option<Vec<u8>>, GitError> {
        match Self::execute(args, cwd).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e)
                if e.code == GitErrorCode::CommandFailed
                    && Self::is_not_a_repository(&e.message) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
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
            Err(e)
                if e.code == GitErrorCode::CommandFailed
                    && Self::is_not_a_repository(&e.message) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_status(path: &str) -> Result<RepositoryStatus, GitError> {
        let root = match Self::is_repository(path).await? {
            Some(root) => root,
            None => return Ok(Self::not_a_repo_status()),
        };

        let output =
            match Self::execute_optional(&["status", "--porcelain=v1", "--branch", "-z", "-uall"], &root)
                .await?
            {
                Some(bytes) => bytes,
                None => return Ok(Self::not_a_repo_status()),
            };

        let parsed = Self::parse_status_porcelain_v1_z(&output)?;

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
            let name = parts.next().unwrap_or_default().trim();
            if name.is_empty() {
                continue;
            }
            let head = parts.next().unwrap_or_default().trim();
            let upstream = parts.next().unwrap_or_default().trim();

            let is_current = head == "*";
            let upstream_opt = (!upstream.is_empty()).then(|| upstream.to_string());

            if is_current {
                current_local = Some((name.to_string(), upstream_opt.clone()));
            }

            branches.push(BranchInfo {
                name: name.to_string(),
                is_current,
                is_remote: false,
                upstream: upstream_opt,
                ahead: None,
                behind: None,
            });
        }

        for line in remotes.lines() {
            let name = line.trim();
            if name.is_empty() || name.ends_with("/HEAD") {
                continue;
            }
            branches.push(BranchInfo {
                name: name.to_string(),
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

    pub async fn get_commits(
        path: &str,
        limit: u32,
        skip: u32,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        let limit = limit.clamp(1, 200);
        // Use -z for NUL-separated records, %x1f for field separator, %s for subject (no newlines)
        let format = "%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%D%x1f%P%x1f%s";
        let n_arg = format!("-n{limit}");
        let skip_arg = format!("--skip={skip}");
        let pretty_arg = format!("--pretty=format:{format}");
        let args = [
            "log",
            "-z",
            n_arg.as_str(),
            skip_arg.as_str(),
            "--date=iso-strict",
            pretty_arg.as_str(),
        ];

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

    pub async fn get_diff(
        path: &str,
        file_path: &str,
        staged: bool,
    ) -> Result<DiffContent, GitError> {
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
        let range = format!("{commit_hash}^..{commit_hash}");
        let args = ["diff", "--no-color", "--unified=3", &range, "--", file_path];

        let output = match Self::execute(&args, &root).await {
            Ok(bytes) => bytes,
            Err(e) if e.code == GitErrorCode::CommandFailed => {
                // If commit has no parent (initial commit), diff against empty tree
                let args = [
                    "show",
                    "--no-color",
                    "--unified=3",
                    "--format=",
                    commit_hash,
                    "--",
                    file_path,
                ];
                Self::execute(&args, &root).await?
            }
            Err(e) => return Err(e),
        };

        Ok(Self::parse_unified_diff(file_path, &output))
    }

    pub async fn get_commit_files(
        path: &str,
        commit_hash: &str,
    ) -> Result<Vec<CommitFileChange>, GitError> {
        let root = Self::ensure_repo_root(path).await?;

        // Use -m to show files for merge commits, --first-parent to show diff against first parent only
        let output = Self::execute_text(
            &[
                "show",
                "-m",
                "--first-parent",
                "--numstat",
                "--format=",
                "--no-color",
                commit_hash,
            ],
            &root,
        )
        .await?;

        let status_output = Self::execute_text(
            &[
                "show",
                "-m",
                "--first-parent",
                "--name-status",
                "--format=",
                "--no-color",
                commit_hash,
            ],
            &root,
        )
        .await?;

        let mut files: Vec<CommitFileChange> = Vec::new();

        // Parse numstat for additions/deletions
        let mut numstat_map: std::collections::HashMap<String, (Option<u32>, Option<u32>)> =
            std::collections::HashMap::new();
        for line in output.lines() {
            let mut parts = line.splitn(3, '\t');
            let additions = Self::parse_numstat_field(parts.next(), "additions")?;
            let deletions = Self::parse_numstat_field(parts.next(), "deletions")?;
            let Some(file_path) = parts.next() else {
                continue;
            };
            if file_path.is_empty() {
                continue;
            }
            numstat_map.insert(file_path.to_string(), (additions, deletions));
        }

        // Parse name-status for file status
        for line in status_output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split('\t');
            let status_field = parts.next().unwrap_or_default();
            let status_char = status_field.chars().next().unwrap_or(' ');
            let (status, file_path, old_path) = match status_char {
                'A' => {
                    let path = parts.next().unwrap_or_default();
                    if path.is_empty() {
                        continue;
                    }
                    (FileChangeStatus::Added, path.to_string(), None)
                }
                'M' => {
                    let path = parts.next().unwrap_or_default();
                    if path.is_empty() {
                        continue;
                    }
                    (FileChangeStatus::Modified, path.to_string(), None)
                }
                'D' => {
                    let path = parts.next().unwrap_or_default();
                    if path.is_empty() {
                        continue;
                    }
                    (FileChangeStatus::Deleted, path.to_string(), None)
                }
                'R' => {
                    let old = parts.next().unwrap_or_default();
                    let new = parts.next().unwrap_or_default();
                    if old.is_empty() || new.is_empty() {
                        continue;
                    }
                    (
                        FileChangeStatus::Renamed,
                        new.to_string(),
                        Some(old.to_string()),
                    )
                }
                'C' => {
                    let old = parts.next().unwrap_or_default();
                    let new = parts.next().unwrap_or_default();
                    if old.is_empty() || new.is_empty() {
                        continue;
                    }
                    (
                        FileChangeStatus::Copied,
                        new.to_string(),
                        Some(old.to_string()),
                    )
                }
                _ => continue,
            };

            let (additions, deletions) =
                numstat_map.get(&file_path).copied().unwrap_or((None, None));

            files.push(CommitFileChange {
                path: file_path,
                status,
                old_path,
                additions,
                deletions,
                is_binary: additions.is_none() && deletions.is_none(),
            });
        }

        Ok(files)
    }

    pub async fn stage_paths(path: &str, paths: &[String]) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_with_paths(&["add", "--"], paths, &root).await
    }

    pub async fn stage_all(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["add", "-A"], &root).await
    }

    pub async fn unstage_paths(path: &str, paths: &[String]) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_with_paths(&["reset", "--"], paths, &root).await
    }

    pub async fn unstage_all(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["reset"], &root).await
    }

    pub async fn discard_worktree_paths(path: &str, paths: &[String]) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_with_paths(&["checkout", "--"], paths, &root).await
    }

    pub async fn discard_worktree_all(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["checkout", "--", "."], &root).await
    }

    pub async fn clean_paths(path: &str, paths: &[String]) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_with_paths(&["clean", "-f", "--"], paths, &root).await
    }

    pub async fn clean_all(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["clean", "-fd"], &root).await
    }

    pub async fn commit(path: &str, message: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["commit", "-m", message], &root).await
    }

    pub async fn push(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["push"], &root).await
    }

    pub async fn pull(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["pull"], &root).await
    }

    pub async fn fetch(path: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["fetch"], &root).await
    }

    pub async fn checkout_branch(path: &str, branch: &str) -> Result<(), GitError> {
        let root = Self::ensure_repo_root(path).await?;
        Self::execute_no_output(&["checkout", branch], &root).await
    }

    pub async fn init_repo(path: &str) -> Result<(), GitError> {
        Self::execute_no_output(&["init"], path).await
    }

    async fn get_ahead_behind(
        cwd: &str,
        branch: &str,
        upstream: &str,
    ) -> Result<(u32, u32), GitError> {
        let range = format!("{branch}...{upstream}");
        let output =
            Self::execute_text(&["rev-list", "--left-right", "--count", &range], cwd).await?;
        let mut parts = output.split_whitespace();
        let ahead = parts
            .next()
            .ok_or_else(|| GitError::parse_error("Missing ahead count"))?
            .parse::<u32>()
            .map_err(|e| GitError::parse_error(format!("Invalid ahead count: {e}")))?;
        let behind = parts
            .next()
            .ok_or_else(|| GitError::parse_error("Missing behind count"))?
            .parse::<u32>()
            .map_err(|e| GitError::parse_error(format!("Invalid behind count: {e}")))?;
        Ok((ahead, behind))
    }

    fn parse_numstat_field(
        field: Option<&str>,
        name: &'static str,
    ) -> Result<Option<u32>, GitError> {
        let Some(field) = field else {
            return Err(GitError::parse_error(format!(
                "Missing numstat field: {name}"
            )));
        };
        let field = field.trim();
        if field == "-" {
            return Ok(None);
        }
        if field.is_empty() {
            return Err(GitError::parse_error(format!(
                "Empty numstat field: {name}"
            )));
        }
        let value = field
            .parse::<u32>()
            .map_err(|e| GitError::parse_error(format!("Invalid numstat {name}: {e}")))?;
        Ok(Some(value))
    }

    fn parse_status_porcelain_v1_z(bytes: &[u8]) -> Result<ParsedStatus, GitError> {
        let mut parsed = ParsedStatus::default();

        let mut entries = bytes
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .peekable();

        while let Some(entry) = entries.next() {
            // 安全检查：分支信息行至少需要 3 个字节 ("## ")
            if entry.starts_with(b"## ") && entry.len() > 3 {
                let line = String::from_utf8_lossy(&entry[3..]);
                Self::parse_branch_summary(line.trim(), &mut parsed);
                continue;
            }

            // 状态行至少需要 3 个字节 (XY + 空格 + 路径)
            if entry.len() < 3 {
                return Err(GitError::parse_error(
                    "Invalid porcelain entry (too short)".to_string(),
                ));
            }

            let x = entry[0] as char;
            let y = entry[1] as char;

            let path_part = String::from_utf8_lossy(&entry[2..]).trim().to_string();
            if path_part.is_empty() {
                return Err(GitError::parse_error(
                    "Invalid porcelain entry (missing path)".to_string(),
                ));
            }

            if x == '?' && y == '?' {
                parsed.untracked_files.push(FileChange {
                    path: path_part,
                    status: FileChangeStatus::Untracked,
                    old_path: None,
                });
                continue;
            }

            let is_unmerged = matches!(
                (x, y),
                ('D', 'D')
                    | ('A', 'U')
                    | ('U', 'D')
                    | ('U', 'A')
                    | ('D', 'U')
                    | ('A', 'A')
                    | ('U', 'U')
            );

            if is_unmerged || x == 'U' || y == 'U' {
                parsed.conflicted_files.push(FileChange {
                    path: path_part,
                    status: FileChangeStatus::Conflicted,
                    old_path: None,
                });
                continue;
            }

            let is_rename_or_copy = x == 'R' || x == 'C' || y == 'R' || y == 'C';
            let (old_path, new_path) = if is_rename_or_copy {
                let old = path_part;
                let Some(new) = entries.next() else {
                    return Err(GitError::parse_error(
                        "Invalid rename/copy porcelain entry (missing new path)".to_string(),
                    ));
                };
                let new = String::from_utf8_lossy(new).to_string();
                if new.trim().is_empty() {
                    return Err(GitError::parse_error(
                        "Invalid rename/copy porcelain entry (empty new path)".to_string(),
                    ));
                }
                (Some(old), new.trim().to_string())
            } else {
                (None, path_part)
            };

            let staged_status = (x != ' ').then(|| Self::map_status_char(x)).flatten();
            let modified_status = (y != ' ').then(|| Self::map_status_char(y)).flatten();

            match (staged_status, modified_status) {
                (Some(staged), Some(modified)) => {
                    parsed.staged_files.push(FileChange {
                        path: new_path.clone(),
                        status: staged,
                        old_path: old_path.clone(),
                    });
                    parsed.modified_files.push(FileChange {
                        path: new_path,
                        status: modified,
                        old_path,
                    });
                }
                (Some(staged), None) => {
                    parsed.staged_files.push(FileChange {
                        path: new_path,
                        status: staged,
                        old_path,
                    });
                }
                (None, Some(modified)) => {
                    parsed.modified_files.push(FileChange {
                        path: new_path,
                        status: modified,
                        old_path,
                    });
                }
                (None, None) => {}
            }
        }

        Ok(parsed)
    }

    fn map_status_char(ch: char) -> Option<FileChangeStatus> {
        match ch {
            'A' => Some(FileChangeStatus::Added),
            'M' => Some(FileChangeStatus::Modified),
            'D' => Some(FileChangeStatus::Deleted),
            'R' => Some(FileChangeStatus::Renamed),
            'C' => Some(FileChangeStatus::Copied),
            'T' => Some(FileChangeStatus::TypeChanged),
            '?' => Some(FileChangeStatus::Untracked),
            '!' => None,
            ' ' => None,
            _ => Some(FileChangeStatus::Unknown),
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
            let mut fields = record.split(|b| *b == 0x1f);
            let Some(hash) = fields.next() else {
                continue;
            };
            let Some(short_hash) = fields.next() else {
                continue;
            };
            let Some(author_name) = fields.next() else {
                continue;
            };
            let Some(author_email) = fields.next() else {
                continue;
            };
            let Some(date) = fields.next() else {
                continue;
            };
            let Some(refs_field) = fields.next() else {
                continue;
            };
            let Some(parents_field) = fields.next() else {
                continue;
            };
            let Some(message) = fields.next() else {
                continue;
            };

            let refs_str = String::from_utf8_lossy(refs_field);
            let refs = Self::parse_refs(refs_str.as_ref());

            let parents_str = String::from_utf8_lossy(parents_field);
            let parents: Vec<String> = parents_str
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            commits.push(CommitInfo {
                hash: String::from_utf8_lossy(hash).to_string(),
                short_hash: String::from_utf8_lossy(short_hash).to_string(),
                author_name: String::from_utf8_lossy(author_name).to_string(),
                author_email: String::from_utf8_lossy(author_email).to_string(),
                date: String::from_utf8_lossy(date).to_string(),
                message: String::from_utf8_lossy(message).to_string(),
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
                Some('+') => (DiffLineType::Added, old_line, new_line.map(|n| n + 1)),
                Some('-') => (DiffLineType::Removed, old_line.map(|o| o + 1), new_line),
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

        let mut parts = header.split_whitespace();
        let _at = parts.next();
        let Some(old) = parts.next() else {
            return (None, None);
        };
        let Some(new) = parts.next() else {
            return (None, None);
        };

        if let Some(old_part) = old.strip_prefix('-') {
            old_start = old_part
                .split(',')
                .next()
                .and_then(|s| s.parse::<u32>().ok());
        }

        if let Some(new_part) = new.strip_prefix('+') {
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
