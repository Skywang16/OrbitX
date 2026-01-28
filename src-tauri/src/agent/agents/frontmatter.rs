use std::collections::HashMap;

use crate::agent::agents::AgentMode;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::permissions::ToolFilter;

#[derive(Debug, Default)]
pub struct ParsedFrontmatter {
    pub fields: HashMap<String, String>,
}

pub fn split_frontmatter(markdown: &str) -> (Option<&str>, &str) {
    let mut lines = markdown.lines();
    if lines.next() != Some("---") {
        return (None, markdown);
    }

    let mut end_idx = None;
    let mut offset = 4usize;
    for line in lines {
        if line.trim() == "---" {
            end_idx = Some(offset);
            break;
        }
        offset += line.len() + 1;
    }

    let Some(end) = end_idx else {
        return (None, markdown);
    };

    let (front, rest) = markdown.split_at(end);
    let rest = rest.strip_prefix("---").unwrap_or(rest);
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let front = front.strip_prefix("---\n").unwrap_or(front);
    (Some(front), rest)
}

pub fn parse_frontmatter(raw: &str) -> ParsedFrontmatter {
    let mut parsed = ParsedFrontmatter::default();

    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            continue;
        }
        if trimmed.trim_start().starts_with('#') {
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        parsed.fields.insert(key, value);
    }

    parsed
}

pub fn parse_agent_mode(raw: &str) -> AgentResult<AgentMode> {
    match raw.trim() {
        "primary" => Ok(AgentMode::Primary),
        "subagent" => Ok(AgentMode::Subagent),
        "internal" => Ok(AgentMode::Internal),
        other => Err(AgentError::Parse(format!("Unknown agent mode: {other}"))),
    }
}

/// Parse tool filter from frontmatter.
///
/// Claude Code subagent format:
/// ```yaml
/// tools: Read, Grep, List        # whitelist (comma-separated or YAML list)
/// disallowedTools: Write, Shell  # blacklist
/// ```
///
/// If neither is specified, returns allow-all filter (inherit from parent).
pub fn parse_tool_filter(frontmatter: &str) -> AgentResult<ToolFilter> {
    let tools = parse_tools_field(frontmatter, "tools")?;
    let disallowed = parse_tools_field(frontmatter, "disallowedTools")?
        .or_else(|| parse_tools_field(frontmatter, "disallowed_tools").ok().flatten());

    match (tools, disallowed) {
        (None, None) => Ok(ToolFilter::allow_all()),
        (Some(whitelist), None) => Ok(ToolFilter::whitelist(whitelist)),
        (None, Some(blacklist)) => Ok(ToolFilter::blacklist(blacklist)),
        (Some(whitelist), Some(blacklist)) => {
            Ok(ToolFilter::whitelist(whitelist).with_disallowed(blacklist))
        }
    }
}

/// Parse a tools field (either `tools:` or `disallowedTools:`).
///
/// Supports two formats:
/// 1. Inline comma-separated: `tools: Read, Grep, List`
/// 2. YAML list:
///    ```yaml
///    tools:
///      - Read
///      - Grep
///      - List
///    ```
fn parse_tools_field(frontmatter: &str, field_name: &str) -> AgentResult<Option<Vec<String>>> {
    let field_prefix = format!("{}:", field_name);
    let mut in_list = false;
    let mut tools: Vec<String> = Vec::new();

    for line in frontmatter.lines() {
        let raw = line.trim_end();
        if raw.trim().is_empty() {
            continue;
        }
        if raw.trim_start().starts_with('#') {
            continue;
        }

        let indent = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = raw.trim();

        // Check for field start
        if indent == 0 && trimmed.to_lowercase().starts_with(&field_prefix.to_lowercase()) {
            let value = trimmed[field_prefix.len()..].trim();

            // Inline format: `tools: Read, Grep, List`
            if !value.is_empty() {
                let parsed: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !parsed.is_empty() {
                    return Ok(Some(parsed));
                }
            }

            // Start YAML list mode
            in_list = true;
            continue;
        }

        if !in_list {
            continue;
        }

        // End of list (another top-level field)
        if indent == 0 {
            break;
        }

        // Parse YAML list item: `  - Read`
        if let Some(item) = trimmed.strip_prefix("- ") {
            let tool = item.trim().trim_matches('"').trim_matches('\'').to_lowercase();
            if !tool.is_empty() {
                tools.push(tool);
            }
        }
    }

    if tools.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tools))
    }
}
