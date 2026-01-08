//! Command line normalization and key extraction for completion learning.
//!
//! 目标：
//! - 把“用户实际输入的命令”从 prompt/噪声里剥离出来
//! - 生成稳定的小 key（控制状态空间，保证学习模型体积可控）

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandKey {
    pub key: String,
    pub root: String,
    pub sub: Option<String>,
}

pub fn normalize_command_line(raw: &str) -> Option<String> {
    let line = raw.lines().last()?.trim();
    if line.is_empty() {
        return None;
    }

    // 常见 prompt 格式：`user@host % cmd` / `user@host $ cmd`
    // 取最后一个分隔符后的部分，避免把 prompt 当成命令。
    let mut best_start: Option<usize> = None;
    for delim in [" % ", " $ ", " # ", " > "] {
        if let Some(idx) = line.rfind(delim) {
            best_start =
                Some(best_start.map_or(idx + delim.len(), |prev| prev.max(idx + delim.len())));
        }
    }

    if let Some(start) = best_start {
        let cmd = line[start..].trim();
        if !cmd.is_empty() {
            return Some(cmd.to_string());
        }
    }

    // 兜底：没有空格分隔时，尝试按最后一个提示符字符截断
    for ch in ['%', '$', '#', '>'] {
        if let Some(idx) = line.rfind(ch) {
            let cmd = line[idx + 1..].trim();
            if !cmd.is_empty() {
                return Some(cmd.to_string());
            }
        }
    }

    Some(line.to_string())
}

pub fn extract_command_key(raw_command_line: &str) -> Option<CommandKey> {
    let command_line = normalize_command_line(raw_command_line)?;
    let cleaned = strip_leading_noise(&command_line);
    let tokens = tokenize_simple(&cleaned);
    if tokens.is_empty() {
        return None;
    }

    let root = tokens[0].to_string();
    let sub = extract_subcommand(&root, &tokens);
    let key = match &sub {
        Some(sub) => format!("{root} {sub}"),
        None => root.clone(),
    };

    Some(CommandKey { key, root, sub })
}

fn strip_leading_noise(command_line: &str) -> String {
    let mut line = command_line.trim();

    // 常见：sudo
    if let Some(rest) = line.strip_prefix("sudo ") {
        line = rest.trim_start();
    }

    // 环境变量赋值前缀：FOO=bar cmd
    // 只做最保守的处理：连续的 `NAME=...` 且 NAME 没有 '/'。
    loop {
        let Some(first) = line.split_whitespace().next() else {
            break;
        };
        let Some(eq_pos) = first.find('=') else { break };
        let name = &first[..eq_pos];
        if name.is_empty() || name.contains('/') {
            break;
        }

        if let Some(rest) = line.strip_prefix(first) {
            line = rest.trim_start();
            continue;
        }
        break;
    }

    line.to_string()
}

fn tokenize_simple(command_line: &str) -> Vec<&str> {
    command_line.split_whitespace().collect()
}

fn extract_subcommand(root: &str, tokens: &[&str]) -> Option<String> {
    // 控制状态空间：只对“明确存在子命令语义”的命令取第二个 token
    // 并且跳过以 '-' 开头的 option。
    if tokens.len() < 2 {
        return None;
    }

    let takes_sub = matches!(
        root,
        "git"
            | "docker"
            | "kubectl"
            | "cargo"
            | "npm"
            | "pnpm"
            | "yarn"
            | "go"
            | "brew"
            | "systemctl"
            | "journalctl"
    );
    if !takes_sub {
        return None;
    }

    let candidate = tokens[1];
    if candidate.starts_with('-') {
        return None;
    }

    Some(candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_prompt_command_line() {
        let raw = "user@host % git status";
        assert_eq!(normalize_command_line(raw).as_deref(), Some("git status"));
    }

    #[test]
    fn extracts_git_subcommand_key() {
        let key = extract_command_key("git status").unwrap();
        assert_eq!(key.key, "git status");
        assert_eq!(key.root, "git");
        assert_eq!(key.sub.as_deref(), Some("status"));
    }

    #[test]
    fn extracts_non_subcommand_root_only() {
        let key = extract_command_key("ls -la").unwrap();
        assert_eq!(key.key, "ls");
        assert_eq!(key.root, "ls");
        assert_eq!(key.sub, None);
    }

    #[test]
    fn strips_sudo_prefix() {
        let key = extract_command_key("sudo git status").unwrap();
        assert_eq!(key.key, "git status");
    }
}
