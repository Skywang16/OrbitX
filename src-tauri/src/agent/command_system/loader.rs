use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::agent::agents::frontmatter::{parse_frontmatter, split_frontmatter};
use crate::agent::error::{AgentError, AgentResult};

use super::types::{CommandConfig, CommandRenderResult, CommandSummary};

pub struct CommandConfigLoader;

impl CommandConfigLoader {
    pub async fn load_for_workspace(
        workspace_root: &Path,
    ) -> AgentResult<HashMap<String, CommandConfig>> {
        let mut configs: HashMap<String, CommandConfig> = HashMap::new();

        let dir = workspace_root.join(".orbitx").join("commands");
        let Ok(mut entries) = fs::read_dir(&dir).await else {
            return Ok(configs);
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok(cfg) = Self::load_one(&path).await {
                configs.insert(cfg.name.clone(), cfg);
            }
        }

        Ok(configs)
    }

    pub async fn load_one(path: &PathBuf) -> AgentResult<CommandConfig> {
        let raw = fs::read_to_string(path).await.map_err(AgentError::from)?;

        let (front, body) = split_frontmatter(&raw);
        let front = front.ok_or_else(|| {
            AgentError::Parse(format!(
                "Missing frontmatter in command config {}",
                path.display()
            ))
        })?;

        let parsed = parse_frontmatter(front);
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AgentError::Parse("Invalid command filename".to_string()))?;

        let subtask = parsed
            .fields
            .get("subtask")
            .map(|v| v == "true")
            .unwrap_or(false);

        Ok(CommandConfig {
            name,
            description: parsed.fields.get("description").cloned(),
            agent: parsed.fields.get("agent").cloned(),
            model: parsed.fields.get("model").cloned(),
            subtask,
            template: body.trim().to_string(),
            source_path: Some(path.to_string_lossy().to_string()),
        })
    }

    pub fn summarize(cfg: &CommandConfig) -> CommandSummary {
        CommandSummary {
            name: cfg.name.clone(),
            description: cfg.description.clone(),
            agent: cfg.agent.clone(),
            model: cfg.model.clone(),
            subtask: cfg.subtask,
        }
    }

    pub fn render(cfg: &CommandConfig, input: &str) -> CommandRenderResult {
        let prompt = cfg.template.replace("{{input}}", input);
        CommandRenderResult {
            name: cfg.name.clone(),
            agent: cfg.agent.clone(),
            model: cfg.model.clone(),
            subtask: cfg.subtask,
            prompt,
        }
    }

    pub fn render_with_skills(
        cfg: &CommandConfig,
        input: &str,
        skills: &[crate::agent::skill::SkillContent],
    ) -> CommandRenderResult {
        let prompt = cfg.template.replace("{{input}}", input);
        let prompt = expand_skill_refs(&prompt, skills);
        CommandRenderResult {
            name: cfg.name.clone(),
            agent: cfg.agent.clone(),
            model: cfg.model.clone(),
            subtask: cfg.subtask,
            prompt,
        }
    }
}

fn expand_skill_refs(input: &str, skills: &[crate::agent::skill::SkillContent]) -> String {
    // Template syntax: {{skill:name}}
    let mut out = input.to_string();
    for skill in skills {
        let needle = format!("{{{{skill:{}}}}}", skill.metadata.name);
        if out.contains(&needle) {
            out = out.replace(&needle, skill.instructions.trim());
        }
    }
    out
}
