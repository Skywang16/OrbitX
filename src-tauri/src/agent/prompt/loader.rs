//! Prompt loader - 从文件系统加载提示词
//!
//! 支持两种加载方式：
//! 1. 编译时嵌入（builtin）- 用于默认提示词
//! 2. 运行时加载（workspace）- 用于用户自定义覆盖

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// 编译时嵌入的内置提示词
pub struct BuiltinPrompts;

impl BuiltinPrompts {
    // === Base prompts ===
    pub fn role() -> &'static str {
        include_str!("../../../prompts/base/role.md")
    }

    pub fn rules() -> &'static str {
        include_str!("../../../prompts/base/rules.md")
    }

    pub fn methodology() -> &'static str {
        include_str!("../../../prompts/base/methodology.md")
    }

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

    pub fn agent_general() -> &'static str {
        include_str!("../../../prompts/agents/general.md")
    }

    pub fn agent_research() -> &'static str {
        include_str!("../../../prompts/agents/research.md")
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

    pub fn system_subtask_summary_user() -> &'static str {
        include_str!("../../../prompts/system/subtask_summary_user.md")
    }

    pub fn system_conversation_summary() -> &'static str {
        include_str!("../../../prompts/system/conversation_summary.md")
    }
}

/// 运行时提示词加载器
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

    /// 加载提示词，优先使用 workspace 覆盖，否则使用 builtin
    pub async fn load(&mut self, category: &str, name: &str) -> Option<String> {
        let cache_key = format!("{}/{}", category, name);

        // 检查缓存
        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        // 尝试从 workspace 加载
        if let Some(ref workspace) = self.workspace_path {
            let workspace_file = Path::new(workspace)
                .join(".orbitx")
                .join("prompts")
                .join(category)
                .join(format!("{}.md", name));

            if let Ok(content) = fs::read_to_string(&workspace_file).await {
                self.cache.insert(cache_key, content.clone());
                return Some(content);
            }
        }

        // 回退到 builtin
        let builtin = match (category, name) {
            ("base", "role") => Some(BuiltinPrompts::role()),
            ("base", "rules") => Some(BuiltinPrompts::rules()),
            ("base", "methodology") => Some(BuiltinPrompts::methodology()),
            ("agents", "coder") => Some(BuiltinPrompts::agent_coder()),
            ("agents", "plan") => Some(BuiltinPrompts::agent_plan()),
            ("agents", "explore") => Some(BuiltinPrompts::agent_explore()),
            ("agents", "general") => Some(BuiltinPrompts::agent_general()),
            ("agents", "research") => Some(BuiltinPrompts::agent_research()),
            ("reminders", "plan_mode") => Some(BuiltinPrompts::reminder_plan_mode()),
            ("reminders", "coder_with_plan") => Some(BuiltinPrompts::reminder_coder_with_plan()),
            ("reminders", "no_workspace") => Some(BuiltinPrompts::reminder_no_workspace()),
            ("reminders", "loop_warning") => Some(BuiltinPrompts::reminder_loop_warning()),
            ("reminders", "duplicate_tools") => Some(BuiltinPrompts::reminder_duplicate_tools()),
            ("system", "env") => Some(BuiltinPrompts::system_env()),
            ("system", "compaction") => Some(BuiltinPrompts::system_compaction()),
            ("system", "subtask_summary_user") => Some(BuiltinPrompts::system_subtask_summary_user()),
            ("system", "conversation_summary") => {
                Some(BuiltinPrompts::system_conversation_summary())
            }
            _ => None,
        };

        if let Some(content) = builtin {
            self.cache.insert(cache_key, content.to_string());
            return Some(content.to_string());
        }

        None
    }

    /// 加载 workspace 自定义 agent（.orbitx/agents/*.md）
    pub async fn load_workspace_agents(&self) -> HashMap<String, String> {
        let mut agents = HashMap::new();

        let Some(ref workspace) = self.workspace_path else {
            return agents;
        };

        let agents_dir = Path::new(workspace).join(".orbitx").join("agents");
        let Ok(mut entries) = fs::read_dir(&agents_dir).await else {
            return agents;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path).await {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    agents.insert(name.to_string(), content);
                }
            }
        }

        agents
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_prompts_exist() {
        assert!(!BuiltinPrompts::role().is_empty());
        assert!(!BuiltinPrompts::rules().is_empty());
        assert!(!BuiltinPrompts::methodology().is_empty());
        assert!(!BuiltinPrompts::agent_coder().is_empty());
    }
}
