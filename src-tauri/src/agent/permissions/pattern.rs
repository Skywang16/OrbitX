use crate::agent::permissions::types::ToolAction;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PermissionPattern {
    pub tool: String,
    pub param: Option<String>,
}

fn normalize_tool_name(raw: &str) -> String {
    // Backward compatible with older settings and agent frontmatter:
    // - Case-insensitive tool names (Read vs read)
    // - Legacy aliases (Bash/WebFetch) vs current tool identifiers
    let lower = raw.trim().to_ascii_lowercase();
    if let Some(mapped) = map_tool_alias_with_optional_glob(&lower, "bash", "shell") {
        return mapped;
    }
    if let Some(mapped) = map_tool_alias_with_optional_glob(&lower, "webfetch", "web_fetch") {
        return mapped;
    }

    match lower.as_str() {
        "readfile" => "read".to_string(),
        "writefile" => "write".to_string(),
        "editfile" => "edit".to_string(),
        "listfiles" => "list".to_string(),
        other => other.to_string(),
    }
}

fn map_tool_alias_with_optional_glob(lower: &str, from: &str, to: &str) -> Option<String> {
    if lower == from {
        return Some(to.to_string());
    }
    if lower.starts_with(from) {
        let rest = &lower[from.len()..];
        if !rest.is_empty() && rest.chars().all(|c| c == '*' || c == '?') {
            return Some(format!("{to}{rest}"));
        }
    }
    None
}

impl PermissionPattern {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        let Some(open_idx) = raw.find('(') else {
            return Some(Self {
                tool: normalize_tool_name(raw),
                param: None,
            });
        };

        let close_idx = raw.rfind(')')?;

        if close_idx <= open_idx {
            return None;
        }

        // 安全切片
        let tool = raw.get(..open_idx).map(|s| s.trim()).unwrap_or("");
        if tool.is_empty() {
            return None;
        }

        let param = raw
            .get(open_idx + 1..close_idx)
            .map(|s| s.trim())
            .unwrap_or("");
        let param = if param.is_empty() {
            None
        } else {
            Some(param.to_string())
        };

        Some(Self {
            tool: normalize_tool_name(tool),
            param,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompiledPermissionPattern {
    raw: String,
    tool_re: Regex,
    param_pattern: Option<String>, // 保存原始 param pattern，用于动态展开
}

impl CompiledPermissionPattern {
    pub fn compile(raw: &str) -> Option<Self> {
        let parsed = PermissionPattern::parse(raw)?;
        let tool_re = compile_glob_regex(&parsed.tool, GlobFlavor::General, None)?;

        Some(Self {
            raw: raw.to_string(),
            tool_re,
            param_pattern: parsed.param,
        })
    }

    pub fn matches(&self, action: &ToolAction) -> bool {
        if !self.tool_re.is_match(&action.tool) {
            return false;
        }

        let Some(param_pattern) = &self.param_pattern else {
            return true;
        };

        // 动态展开 ${workspaceFolder} 并编译正则
        let expanded_pattern = expand_placeholders(param_pattern, &action.workspace_root);
        let Some(param_re) = compile_glob_regex(&expanded_pattern, GlobFlavor::Param, None) else {
            return false;
        };

        for candidate in action.param_variants.iter() {
            if param_re.is_match(candidate) {
                return true;
            }
        }

        false
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

#[derive(Debug, Clone, Copy)]
enum GlobFlavor {
    General,
    Param,
}

fn compile_glob_regex(
    pattern: &str,
    flavor: GlobFlavor,
    _workspace: Option<&std::path::Path>,
) -> Option<Regex> {
    let expanded = expand_env_vars(pattern);
    let (expanded, trailing_recursive_dir) = match flavor {
        // Good taste: `${workspaceFolder}/**` should match both the workspace root
        // and anything under it (so listing the root directory doesn't become a special case).
        GlobFlavor::Param => expanded
            .strip_suffix("/**")
            .map(|prefix| (prefix.to_string(), true))
            .unwrap_or((expanded, false)),
        GlobFlavor::General => (expanded, false),
    };

    let mut out = String::with_capacity(expanded.len() * 2 + 10);
    out.push('^');

    let mut chars = expanded.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    let _ = chars.next();
                    out.push_str(".*");
                } else {
                    match flavor {
                        GlobFlavor::General => out.push_str(".*"),
                        GlobFlavor::Param => out.push_str("[^/]*"),
                    }
                }
            }
            '?' => out.push('.'),
            '.' | '+' | '(' | ')' | '|' | '{' | '}' | '[' | ']' | '^' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    if trailing_recursive_dir {
        out.push_str("(?:/.*)?");
    }

    out.push('$');

    Regex::new(&out).ok()
}

fn expand_env_vars(input: &str) -> String {
    let mut out = input.to_string();

    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy();
        out = out.replace("$HOME", &home);
    }

    out
}

fn expand_placeholders(candidate: &str, workspace_root: &std::path::Path) -> String {
    let mut out = candidate.to_string();
    let ws = workspace_root.to_string_lossy();
    out = out.replace("${workspaceFolder}", &ws);
    out = out.replace("${workspace}", &ws);
    out
}
