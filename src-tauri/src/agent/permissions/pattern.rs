use crate::agent::permissions::types::ToolAction;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PermissionPattern {
    pub tool: String,
    pub param: Option<String>,
}

impl PermissionPattern {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        let Some(open_idx) = raw.find('(') else {
            return Some(Self {
                tool: raw.to_string(),
                param: None,
            });
        };

        let Some(close_idx) = raw.rfind(')') else {
            return None;
        };

        if close_idx <= open_idx {
            return None;
        }

        let tool = raw[..open_idx].trim();
        if tool.is_empty() {
            return None;
        }

        let param = raw[open_idx + 1..close_idx].trim();
        let param = if param.is_empty() {
            None
        } else {
            Some(param.to_string())
        };

        Some(Self {
            tool: tool.to_string(),
            param,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompiledPermissionPattern {
    raw: String,
    tool_re: Regex,
    param_re: Option<Regex>,
}

impl CompiledPermissionPattern {
    pub fn compile(raw: &str) -> Option<Self> {
        let parsed = PermissionPattern::parse(raw)?;
        let tool_re = compile_glob_regex(&parsed.tool, GlobFlavor::General)?;
        let param_re = match parsed.param.as_deref() {
            Some(p) => Some(compile_glob_regex(p, GlobFlavor::Param)?),
            None => None,
        };

        Some(Self {
            raw: raw.to_string(),
            tool_re,
            param_re,
        })
    }

    pub fn matches(&self, action: &ToolAction) -> bool {
        if !self.tool_re.is_match(&action.tool) {
            return false;
        }

        let Some(param_re) = &self.param_re else {
            return true;
        };

        for candidate in action.param_variants.iter() {
            let expanded = expand_placeholders(candidate, &action.workspace_root);
            if param_re.is_match(&expanded) {
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

fn compile_glob_regex(pattern: &str, flavor: GlobFlavor) -> Option<Regex> {
    let expanded = expand_env_vars(pattern);
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

