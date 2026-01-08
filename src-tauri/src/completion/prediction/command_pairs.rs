/// 命令序列预测 - 硬编码关联表
///
/// Linus式设计：简单到无法出错的静态配置。
/// 不需要机器学习，不需要复杂算法，一个数组搞定。
/// 命令关联对：(触发命令模式, 建议的后续命令列表)
pub static COMMAND_PAIRS: &[(&str, &[&str])] = &[
    // 网络调试流程
    ("lsof", &["kill", "kill -9", "netstat"]),
    ("netstat", &["kill", "lsof", "ss"]),
    ("ss", &["kill", "lsof"]),
    // Docker 工作流
    (
        "docker ps",
        &["docker stop", "docker logs", "docker exec", "docker rm"],
    ),
    (
        "docker images",
        &["docker rmi", "docker run", "docker pull"],
    ),
    ("docker logs", &["docker restart", "docker stop"]),
    // Git 工作流
    ("git status", &["git add", "git diff", "git restore"]),
    ("git add", &["git commit", "git status", "git reset"]),
    ("git commit", &["git push", "git log", "git show"]),
    ("git pull", &["git status", "git log"]),
    ("git diff", &["git add", "git restore"]),
    // 进程管理
    ("ps aux", &["kill", "kill -9", "pkill"]),
    ("top", &["kill", "pkill"]),
    ("htop", &["kill", "pkill"]),
    // 文件查找
    ("find", &["xargs", "rm", "ls", "cat"]),
    ("grep", &["cat", "less", "vim", "code"]),
    ("ls", &["cd", "cat", "less", "rm", "mv", "cp"]),
    // 包管理 - Node
    ("npm install", &["npm run", "npm start", "npm test"]),
    ("npm run", &["npm test", "git add"]),
    ("npm test", &["git add", "npm run"]),
    // 包管理 - Python
    ("pip install", &["python", "pytest"]),
    ("pytest", &["git add", "python"]),
    // 包管理 - Rust
    (
        "cargo build",
        &["cargo run", "cargo test", "./target/debug"],
    ),
    ("cargo test", &["git add", "cargo build"]),
    ("cargo run", &["cargo build", "cargo test"]),
    // 系统操作
    (
        "systemctl status",
        &["systemctl restart", "systemctl stop", "journalctl"],
    ),
    ("journalctl", &["systemctl restart", "systemctl status"]),
];

/// 检查命令是否匹配某个模式
pub fn matches_command_pattern(executed_cmd: &str, pattern: &str) -> bool {
    // 只匹配“命令词边界”，避免把 `ls` 错配到 `lsblk` 这种垃圾情况。
    //
    // 规则：
    // - pattern 是 1 个词：匹配 executed 的第 1 个词
    // - pattern 是 N 个词：匹配 executed 的前 N 个词
    // - 允许前缀 `sudo`（常见真实场景）
    let mut cmd = executed_cmd.trim();
    if let Some(rest) = cmd.strip_prefix("sudo ") {
        cmd = rest.trim_start();
    }

    let pattern_tokens: Vec<&str> = pattern.split_whitespace().collect();
    if pattern_tokens.is_empty() {
        return false;
    }

    let cmd_tokens: Vec<&str> = cmd.split_whitespace().collect();
    if cmd_tokens.len() < pattern_tokens.len() {
        return false;
    }

    let head = cmd_tokens[..pattern_tokens.len()].join(" ");
    head == pattern
}

/// 获取建议的后续命令
pub fn get_suggested_commands(last_command: &str) -> Option<Vec<String>> {
    for (pattern, suggestions) in COMMAND_PAIRS {
        if matches_command_pattern(last_command, pattern) {
            return Some(suggestions.iter().map(|s| s.to_string()).collect());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsof_kill_prediction() {
        let suggestions = get_suggested_commands("lsof -i :8080");
        assert!(suggestions.is_some());
        let cmds = suggestions.unwrap();
        assert!(cmds.contains(&"kill".to_string()));
        assert!(cmds.contains(&"kill -9".to_string()));
    }

    #[test]
    fn test_git_workflow() {
        // git status → git add
        let suggestions = get_suggested_commands("git status");
        assert!(suggestions.is_some());
        assert!(suggestions.unwrap().contains(&"git add".to_string()));

        // git add → git commit
        let suggestions = get_suggested_commands("git add src/main.rs");
        assert!(suggestions.is_some());
        assert!(suggestions.unwrap().contains(&"git commit".to_string()));
    }

    #[test]
    fn test_docker_workflow() {
        let suggestions = get_suggested_commands("docker ps -a");
        assert!(suggestions.is_some());
        let cmds = suggestions.unwrap();
        assert!(cmds.contains(&"docker stop".to_string()));
        assert!(cmds.contains(&"docker logs".to_string()));
    }

    #[test]
    fn test_no_match() {
        let suggestions = get_suggested_commands("echo hello");
        assert!(suggestions.is_none());
    }
}
