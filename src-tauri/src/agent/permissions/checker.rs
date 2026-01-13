use crate::agent::permissions::pattern::CompiledPermissionPattern;
use crate::agent::permissions::types::{PermissionDecision, ToolAction};
use crate::settings::types::PermissionRules;

#[derive(Debug, Clone)]
pub struct PermissionChecker {
    allow: Vec<CompiledPermissionPattern>,
    deny: Vec<CompiledPermissionPattern>,
    ask: Vec<CompiledPermissionPattern>,
}

impl PermissionChecker {
    pub fn new(rules: &PermissionRules) -> Self {
        Self {
            allow: compile_all(&rules.allow),
            deny: compile_all(&rules.deny),
            ask: compile_all(&rules.ask),
        }
    }

    pub fn check(&self, action: &ToolAction) -> PermissionDecision {
        if matches_any(&self.deny, action) {
            return PermissionDecision::Deny;
        }
        if matches_any(&self.allow, action) {
            return PermissionDecision::Allow;
        }
        if matches_any(&self.ask, action) {
            return PermissionDecision::Ask;
        }
        PermissionDecision::Ask
    }
}

fn compile_all(patterns: &[String]) -> Vec<CompiledPermissionPattern> {
    patterns
        .iter()
        .filter_map(|p| CompiledPermissionPattern::compile(p))
        .collect()
}

fn matches_any(patterns: &[CompiledPermissionPattern], action: &ToolAction) -> bool {
    patterns.iter().any(|p| p.matches(action))
}
