use std::collections::HashMap;

use crate::agent::agents::{AgentMode, AgentPermissionConfig};
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::permissions::PermissionDecision;

#[derive(Debug, Default)]
pub struct ParsedFrontmatter {
    pub fields: HashMap<String, String>,
    pub permission_fields: HashMap<String, String>,
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
    let mut current_section: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            continue;
        }
        if trimmed.trim_start().starts_with('#') {
            continue;
        }

        let indent = line.chars().take_while(|c| *c == ' ').count();
        if indent == 0 {
            current_section = None;
        }

        if indent == 0 && trimmed.ends_with(':') && !trimmed.contains(' ') {
            current_section = Some(trimmed.trim_end_matches(':').to_string());
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();

        if current_section.as_deref() == Some("permission") {
            parsed.permission_fields.insert(key, value);
        } else {
            parsed.fields.insert(key, value);
        }
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

pub fn parse_permission_decision(raw: &str) -> AgentResult<PermissionDecision> {
    match raw.trim() {
        "allow" => Ok(PermissionDecision::Allow),
        "deny" => Ok(PermissionDecision::Deny),
        "ask" => Ok(PermissionDecision::Ask),
        other => Err(AgentError::Parse(format!(
            "Unknown permission decision: {other}"
        ))),
    }
}

pub fn build_permission_config(parsed: &ParsedFrontmatter) -> AgentResult<AgentPermissionConfig> {
    let default = parsed
        .permission_fields
        .get("*")
        .map(|s| parse_permission_decision(s))
        .transpose()?
        .unwrap_or(PermissionDecision::Ask);

    let mut cfg = AgentPermissionConfig {
        default,
        ..Default::default()
    };

    macro_rules! set_opt {
        ($field:ident, $key:literal) => {
            if let Some(value) = parsed.permission_fields.get($key) {
                cfg.$field = Some(parse_permission_decision(value)?);
            }
        };
    }

    set_opt!(read, "read");
    set_opt!(edit, "edit");
    set_opt!(shell, "shell");
    set_opt!(grep, "grep");
    set_opt!(list, "list");
    set_opt!(web_fetch, "web_fetch");
    set_opt!(task, "task");
    set_opt!(semantic_search, "semantic_search");
    Ok(cfg)
}

