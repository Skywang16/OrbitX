use crate::agent::permissions::pattern::CompiledPermissionPattern;
use crate::agent::permissions::types::{PermissionDecision, ToolAction};
use crate::settings::types::PermissionRules;

#[derive(Debug, Clone)]
struct CompiledPermissionRule {
    decision: PermissionDecision,
    pattern: CompiledPermissionPattern,
}

/// Settings permission checker using Claude Code priority semantics.
///
/// Evaluation order: deny > ask > allow (rule order within each category doesn't matter).
#[derive(Debug, Clone)]
pub struct PermissionChecker {
    rules: Vec<CompiledPermissionRule>,
    default: PermissionDecision,
}

impl PermissionChecker {
    /// Create from Settings `PermissionRules` (allow/deny/ask arrays).
    pub fn new(rules: &PermissionRules) -> Self {
        let mut compiled = Vec::new();
        compiled.extend(compile_rules(&rules.deny, PermissionDecision::Deny));
        compiled.extend(compile_rules(&rules.ask, PermissionDecision::Ask));
        compiled.extend(compile_rules(&rules.allow, PermissionDecision::Allow));

        Self {
            rules: compiled,
            default: PermissionDecision::Ask,
        }
    }

    /// Check permission using Claude Code priority semantics: deny > ask > allow.
    ///
    /// Evaluation order:
    /// 1. If ANY deny rule matches → Deny (immediate, no further checks)
    /// 2. If ANY ask rule matches → Ask
    /// 3. If ANY allow rule matches → Allow
    /// 4. No matches → return default
    pub fn check(&self, action: &ToolAction) -> PermissionDecision {
        for rule in &self.rules {
            if rule.decision == PermissionDecision::Deny && rule.pattern.matches(action) {
                return PermissionDecision::Deny;
            }
        }

        for rule in &self.rules {
            if rule.decision == PermissionDecision::Ask && rule.pattern.matches(action) {
                return PermissionDecision::Ask;
            }
        }

        for rule in &self.rules {
            if rule.decision == PermissionDecision::Allow && rule.pattern.matches(action) {
                return PermissionDecision::Allow;
            }
        }

        self.default
    }

    /// Check with match status. Returns (decision, did_any_rule_match).
    pub fn check_with_match(&self, action: &ToolAction) -> (PermissionDecision, bool) {
        for rule in &self.rules {
            if rule.decision == PermissionDecision::Deny && rule.pattern.matches(action) {
                return (PermissionDecision::Deny, true);
            }
        }

        for rule in &self.rules {
            if rule.decision == PermissionDecision::Ask && rule.pattern.matches(action) {
                return (PermissionDecision::Ask, true);
            }
        }

        for rule in &self.rules {
            if rule.decision == PermissionDecision::Allow && rule.pattern.matches(action) {
                return (PermissionDecision::Allow, true);
            }
        }

        (self.default, false)
    }
}

fn compile_rules(patterns: &[String], decision: PermissionDecision) -> Vec<CompiledPermissionRule> {
    patterns
        .iter()
        .filter_map(|raw| {
            CompiledPermissionPattern::compile(raw)
                .map(|pattern| CompiledPermissionRule { decision, pattern })
        })
        .collect()
}
