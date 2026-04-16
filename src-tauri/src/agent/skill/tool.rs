use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolDescriptionContext, ToolMetadata, ToolResult, ToolResultContent,
    ToolResultStatus,
};

use super::manager::SkillManager;

/// Skill Tool - Integrates Agent Skills into the standard tool system
///
/// - Skills are registered as standard Tools
/// - Dynamically lists all available skills in the description
/// - LLM activates skills via tool calling
/// - Lazy loading: only loads full content when LLM calls it
pub struct SkillTool {
    manager: Arc<SkillManager>,
}

impl SkillTool {
    pub fn new(manager: Arc<SkillManager>) -> Self {
        Self { manager }
    }

    /// Generate dynamic description containing all available skills
    fn build_description(&self) -> String {
        let skills = self.manager.list_all();

        if skills.is_empty() {
            return "Load a skill to get detailed instructions for a specific task. No skills are currently available.".to_string();
        }

        let mut parts = vec![
            "Load a skill to get detailed instructions for a specific task.".to_string(),
            "Skills provide specialized knowledge and step-by-step guidance.".to_string(),
            "Use this when a task matches an available skill's description.".to_string(),
            "<available_skills>".to_string(),
        ];

        for skill in skills {
            parts.push("  <skill>".to_string());
            parts.push(format!("    <name>{}</name>", skill.name));
            parts.push(format!(
                "    <description>{}</description>",
                skill.description
            ));
            parts.push("  </skill>".to_string());
        }

        parts.push("</available_skills>".to_string());
        parts.join("\n")
    }
}

#[async_trait]
impl RunnableTool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        // Static fallback description
        "Load a skill to get specialized instructions for specific tasks"
    }

    /// Dynamically generate description containing all available skill list
    fn description_with_context(&self, _context: &ToolDescriptionContext) -> Option<String> {
        Some(self.build_description())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The skill identifier from available_skills (e.g., 'code-review' or 'pdf-processing')"
                }
            },
            "required": ["name"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            tags: vec!["knowledge".to_string(), "guidance".to_string()],
            // Skill output should be protected to avoid loss during context compaction
            protected_from_compaction: true,
            ..Default::default()
        }
    }

    async fn run(&self, _context: &AgentRunContext, args: Value) -> ToolExecutorResult<ToolResult> {
        let start_time = std::time::Instant::now();

        // Parse parameters
        let skill_name = args.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolExecutorError::InvalidArguments {
                tool_name: "skill".to_string(),
                error: "Missing required parameter: name".to_string(),
            }
        })?;

        // Check if skill exists
        let metadata = self.manager.get_metadata(skill_name);
        if metadata.is_none() {
            let available: Vec<String> = self
                .manager
                .list_all()
                .iter()
                .map(|s| s.name.to_string())
                .collect();

            return Err(ToolExecutorError::ExecutionFailed {
                tool_name: "skill".to_string(),
                error: format!(
                    "Skill \"{}\" not found. Available skills: {}",
                    skill_name,
                    if available.is_empty() {
                        "none".to_string()
                    } else {
                        available.join(", ")
                    }
                ),
            });
        }

        // Load full content
        let content = self.manager.load_content(skill_name).await.map_err(|e| {
            ToolExecutorError::ExecutionFailed {
                tool_name: "skill".to_string(),
                error: format!("Failed to load skill: {e}"),
            }
        })?;

        // Format output
        let output = format!(
            "## Skill: {}\n\n**Base directory**: {:?}\n\n{}",
            content.metadata.name,
            content.metadata.skill_dir,
            content.instructions.trim()
        );

        // If scripts/references are available, append hints
        let mut hints = Vec::new();
        if !content.scripts.is_empty() {
            hints.push(format!("Available scripts: {}", content.scripts.join(", ")));
        }
        if !content.references.is_empty() {
            hints.push(format!(
                "Available references: {}",
                content.references.join(", ")
            ));
        }

        let final_output = if hints.is_empty() {
            output
        } else {
            format!("{}\n\n---\n\n{}", output, hints.join("\n"))
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(final_output)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: Some(execution_time_ms),
            ext_info: Some(json!({
                "skill_name": skill_name,
                "skill_dir": content.metadata.skill_dir.to_string_lossy(),
                "has_scripts": !content.scripts.is_empty(),
                "has_references": !content.references.is_empty(),
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs;
    use tempfile::TempDir;

    use crate::agent::skill::test_utils::create_test_skill;

    #[tokio::test]
    async fn test_skill_tool_description() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let orbitx_skills = workspace.join(".orbitx/skills");
        std_fs::create_dir_all(&orbitx_skills).unwrap();

        let skill_dir = orbitx_skills.join("test-skill");
        create_test_skill(&skill_dir, "test-skill").unwrap();

        let manager = Arc::new(SkillManager::new());
        manager
            .discover_skills(None, Some(workspace))
            .await
            .unwrap();

        let tool = SkillTool::new(manager);
        let description = tool.build_description();

        assert!(description.contains("<available_skills>"));
        assert!(description.contains("<name>test-skill</name>"));
    }

    #[tokio::test]
    async fn test_skill_tool_empty_description() {
        let manager = Arc::new(SkillManager::new());
        let tool = SkillTool::new(manager);
        let description = tool.build_description();

        assert!(description.contains("No skills are currently available"));
    }
}
