//! Tool filter for Agent capability boundaries.
//!
//! This is separate from `PermissionChecker` which handles user-level allow/deny/ask rules.
//! ToolFilter uses whitelist/blacklist semantics like Claude Code's subagent `tools`/`disallowedTools`.
//!
//! Semantics:
//! - If `tools` (whitelist) is specified: only those tools are available
//! - If `disallowed_tools` (blacklist) is specified: those tools are excluded
//! - Both can be combined: whitelist first, then blacklist exclusions
//! - Empty filter = all tools allowed (inherit from parent)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Tool filter for Agent capability boundaries.
///
/// Unlike PermissionChecker (which uses deny > ask > allow priority for user confirmation),
/// ToolFilter simply determines whether a tool is available to an agent at all.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolFilter {
    /// Whitelist: only these tools are available.
    /// If empty/None, all tools are available (then blacklist applies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashSet<String>>,

    /// Blacklist: these tools are explicitly excluded.
    /// Applied after whitelist filtering.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub disallowed_tools: HashSet<String>,
}

impl ToolFilter {
    /// Create a filter that allows all tools (no restrictions).
    pub fn allow_all() -> Self {
        Self {
            tools: None,
            disallowed_tools: HashSet::new(),
        }
    }

    /// Create a filter with only specified tools allowed (whitelist).
    pub fn whitelist(tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tools: Some(tools.into_iter().map(Into::into).collect()),
            disallowed_tools: HashSet::new(),
        }
    }

    /// Create a filter with specified tools blocked (blacklist).
    pub fn blacklist(tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tools: None,
            disallowed_tools: tools.into_iter().map(Into::into).collect(),
        }
    }

    /// Add tools to the blacklist.
    pub fn with_disallowed(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.disallowed_tools.extend(tools.into_iter().map(Into::into));
        self
    }

    /// Check if a tool is allowed by this filter.
    ///
    /// Returns true if the tool can be used, false if blocked.
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        // Normalize tool name for comparison (lowercase)
        let normalized = tool_name.to_lowercase();

        // Step 1: Check blacklist first (deny always wins)
        if self.disallowed_tools.contains(&normalized) {
            return false;
        }

        // Step 2: Check whitelist (if specified)
        if let Some(whitelist) = &self.tools {
            return whitelist.contains(&normalized);
        }

        // Step 3: No whitelist = allow all (that aren't blacklisted)
        true
    }

    /// Filter a list of tool names, returning only allowed ones.
    pub fn filter_tools<'a>(&self, tools: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        tools.into_iter().filter(|t| self.is_allowed(t)).collect()
    }

    /// Check if this filter has any restrictions.
    pub fn has_restrictions(&self) -> bool {
        self.tools.is_some() || !self.disallowed_tools.is_empty()
    }

    /// Merge with another filter.
    ///
    /// The result is the intersection of capabilities:
    /// - Whitelist: intersection of both (or the one that exists)
    /// - Blacklist: union of both
    pub fn merge(&self, other: &Self) -> Self {
        let tools = match (&self.tools, &other.tools) {
            (None, None) => None,
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (Some(a), Some(b)) => Some(a.intersection(b).cloned().collect()),
        };

        let mut disallowed = self.disallowed_tools.clone();
        disallowed.extend(other.disallowed_tools.iter().cloned());

        Self {
            tools,
            disallowed_tools: disallowed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_all() {
        let filter = ToolFilter::allow_all();
        assert!(filter.is_allowed("read"));
        assert!(filter.is_allowed("write"));
        assert!(filter.is_allowed("shell"));
    }

    #[test]
    fn test_whitelist() {
        let filter = ToolFilter::whitelist(["read", "grep", "list"]);
        assert!(filter.is_allowed("read"));
        assert!(filter.is_allowed("grep"));
        assert!(!filter.is_allowed("write"));
        assert!(!filter.is_allowed("shell"));
    }

    #[test]
    fn test_blacklist() {
        let filter = ToolFilter::blacklist(["write", "shell"]);
        assert!(filter.is_allowed("read"));
        assert!(filter.is_allowed("grep"));
        assert!(!filter.is_allowed("write"));
        assert!(!filter.is_allowed("shell"));
    }

    #[test]
    fn test_whitelist_with_blacklist() {
        let filter = ToolFilter::whitelist(["read", "write", "grep"])
            .with_disallowed(["write"]);
        assert!(filter.is_allowed("read"));
        assert!(filter.is_allowed("grep"));
        assert!(!filter.is_allowed("write")); // blacklisted
        assert!(!filter.is_allowed("shell")); // not in whitelist
    }

    #[test]
    fn test_case_insensitive() {
        let filter = ToolFilter::whitelist(["read", "grep"]);
        assert!(filter.is_allowed("Read"));
        assert!(filter.is_allowed("READ"));
        assert!(filter.is_allowed("read"));
    }

    #[test]
    fn test_merge() {
        let a = ToolFilter::whitelist(["read", "write", "grep"]);
        let b = ToolFilter::whitelist(["read", "grep", "list"]).with_disallowed(["grep"]);
        let merged = a.merge(&b);

        // Whitelist intersection: read, grep
        // Blacklist union: grep
        assert!(merged.is_allowed("read"));
        assert!(!merged.is_allowed("grep")); // blacklisted
        assert!(!merged.is_allowed("write")); // not in intersection
        assert!(!merged.is_allowed("list")); // not in intersection
    }
}
