//! Prompt loader - Load prompts from filesystem
//!
//! Supports two loading methods:
//! 1. Compile-time embedding (builtin) - for default prompts
//! 2. Runtime loading (workspace) - for user custom overrides

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::warn;

/// Compile-time embedded builtin prompts
pub struct BuiltinPrompts;

impl BuiltinPrompts {
    // === Agent prompts ===
    pub fn agent_coder() -> &'static str {
        include_str!("../../../prompts/agents/coder.md")
    }

    pub fn agent_plan() -> &'static str {
        include_str!("../../../prompts/agents/plan.md")
    }

    pub fn agent_explore() -> &'static str {
        include_str!("../../../prompts/agents/explore.md")
    }

    pub fn agent_orchestrate() -> &'static str {
        include_str!("../../../prompts/agents/orchestrate.md")
    }

    pub fn agent_general() -> &'static str {
        include_str!("../../../prompts/agents/general.md")
    }

    pub fn agent_research() -> &'static str {
        include_str!("../../../prompts/agents/research.md")
    }

    pub fn agent_bulk_edit() -> &'static str {
        include_str!("../../../prompts/agents/bulk_edit.md")
    }

    pub fn agent_execute() -> &'static str {
        include_str!("../../../prompts/agents/execute.md")
    }

    // === Reminders ===
    pub fn reminder_plan_mode() -> &'static str {
        include_str!("../../../prompts/reminders/plan_mode.md")
    }

    pub fn reminder_coder_with_plan() -> &'static str {
        include_str!("../../../prompts/reminders/coder_with_plan.md")
    }

    pub fn reminder_no_workspace() -> &'static str {
        include_str!("../../../prompts/reminders/no_workspace.md")
    }

    pub fn reminder_loop_warning() -> &'static str {
        include_str!("../../../prompts/reminders/loop_warning.md")
    }

    pub fn reminder_duplicate_tools() -> &'static str {
        include_str!("../../../prompts/reminders/duplicate_tools.md")
    }

    pub fn reminder_max_steps() -> &'static str {
        include_str!("../../../prompts/reminders/max_steps.md")
    }

    // === System prompts ===
    pub fn system_env() -> &'static str {
        include_str!("../../../prompts/system/env.md")
    }

    pub fn system_compaction() -> &'static str {
        include_str!("../../../prompts/system/compaction.md")
    }

    pub fn system_compaction_user() -> &'static str {
        include_str!("../../../prompts/system/compaction_user.md")
    }

    pub fn system_subagent_summary_user() -> &'static str {
        include_str!("../../../prompts/system/subagent_summary_user.md")
    }

    pub fn system_conversation_summary() -> &'static str {
        include_str!("../../../prompts/system/conversation_summary.md")
    }

    // === Model prompt profiles ===
    pub fn model_openai_codex() -> &'static str {
        include_str!("../../../prompts/models/openai-codex.md")
    }

    pub fn model_openai_o_series() -> &'static str {
        include_str!("../../../prompts/models/openai-o-series.md")
    }

    pub fn model_openai_gpt() -> &'static str {
        include_str!("../../../prompts/models/openai-gpt.md")
    }

    pub fn model_anthropic_claude() -> &'static str {
        include_str!("../../../prompts/models/anthropic-claude.md")
    }

    pub fn model_google_gemini() -> &'static str {
        include_str!("../../../prompts/models/google-gemini.md")
    }

    pub fn model_deepseek() -> &'static str {
        include_str!("../../../prompts/models/deepseek.md")
    }

    pub fn model_generic() -> &'static str {
        include_str!("../../../prompts/models/generic.md")
    }
}

/// Runtime prompt loader
pub struct PromptLoader {
    workspace_path: Option<String>,
    cache: HashMap<String, String>,
}

impl PromptLoader {
    pub fn new(workspace_path: Option<String>) -> Self {
        Self {
            workspace_path,
            cache: HashMap::new(),
        }
    }

    /// Load prompt, prefer workspace override, otherwise use builtin
    pub async fn load(&mut self, category: &str, name: &str) -> Option<String> {
        let cache_key = format!("{category}/{name}");

        // Check cache
        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        // Try loading from workspace
        if let Some(ref workspace) = self.workspace_path {
            let workspace_file = Path::new(workspace)
                .join(".orbitx")
                .join("prompts")
                .join(category)
                .join(format!("{name}.md"));

            match fs::read_to_string(&workspace_file).await {
                Ok(content) => {
                    self.cache.insert(cache_key, content.clone());
                    return Some(content);
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    warn!(
                        "Failed to read workspace prompt '{}': {}",
                        workspace_file.display(),
                        err
                    );
                }
            }
        }

        // Fallback to builtin
        let builtin = match (category, name) {
            ("agents", "coder") => Some(BuiltinPrompts::agent_coder()),
            ("agents", "plan") => Some(BuiltinPrompts::agent_plan()),
            ("agents", "explore") => Some(BuiltinPrompts::agent_explore()),
            ("agents", "orchestrate") => Some(BuiltinPrompts::agent_orchestrate()),
            ("agents", "general") => Some(BuiltinPrompts::agent_general()),
            ("agents", "research") => Some(BuiltinPrompts::agent_research()),
            ("agents", "bulk_edit") => Some(BuiltinPrompts::agent_bulk_edit()),
            ("agents", "execute") => Some(BuiltinPrompts::agent_execute()),
            ("reminders", "plan_mode") => Some(BuiltinPrompts::reminder_plan_mode()),
            ("reminders", "coder_with_plan") => Some(BuiltinPrompts::reminder_coder_with_plan()),
            ("reminders", "no_workspace") => Some(BuiltinPrompts::reminder_no_workspace()),
            ("reminders", "loop_warning") => Some(BuiltinPrompts::reminder_loop_warning()),
            ("reminders", "duplicate_tools") => Some(BuiltinPrompts::reminder_duplicate_tools()),
            ("reminders", "max_steps") => Some(BuiltinPrompts::reminder_max_steps()),
            ("system", "env") => Some(BuiltinPrompts::system_env()),
            ("system", "compaction") => Some(BuiltinPrompts::system_compaction()),
            ("system", "subagent_summary_user") => {
                Some(BuiltinPrompts::system_subagent_summary_user())
            }
            ("system", "conversation_summary") => {
                Some(BuiltinPrompts::system_conversation_summary())
            }
            ("models", "openai-codex") => Some(BuiltinPrompts::model_openai_codex()),
            ("models", "openai-o-series") => Some(BuiltinPrompts::model_openai_o_series()),
            ("models", "openai-gpt") => Some(BuiltinPrompts::model_openai_gpt()),
            ("models", "anthropic-claude") => Some(BuiltinPrompts::model_anthropic_claude()),
            ("models", "google-gemini") => Some(BuiltinPrompts::model_google_gemini()),
            ("models", "deepseek") => Some(BuiltinPrompts::model_deepseek()),
            ("models", "generic") => Some(BuiltinPrompts::model_generic()),
            _ => None,
        };

        if let Some(content) = builtin {
            self.cache.insert(cache_key, content.to_string());
            return Some(content.to_string());
        }

        None
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_prompts_exist() {
        assert!(!BuiltinPrompts::agent_coder().is_empty());
        assert!(!BuiltinPrompts::model_generic().is_empty());
        assert!(!BuiltinPrompts::model_openai_codex().is_empty());
    }
}
