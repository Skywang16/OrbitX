use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::agent::agents::config::{AgentConfig, AgentMode};
use crate::agent::agents::frontmatter::{
    build_permission_config, parse_agent_mode, parse_frontmatter, split_frontmatter,
};
use crate::agent::error::{AgentError, AgentResult};

pub struct AgentConfigLoader;

impl AgentConfigLoader {
    pub fn builtin() -> Vec<AgentConfig> {
        vec![
            AgentConfig {
                name: "coder".to_string(),
                description: Some("Default coding agent".to_string()),
                mode: AgentMode::Primary,
                system_prompt: "You are a coding agent.".to_string(),
                permission: super::config::AgentPermissionConfig::builtin_coder(),
                max_steps: None,
                model_id: None,
                temperature: None,
                top_p: None,
                color: None,
                hidden: false,
                source_path: None,
                is_builtin: true,
            },
            AgentConfig {
                name: "plan".to_string(),
                description: Some("Planning agent (read-only)".to_string()),
                mode: AgentMode::Primary,
                system_prompt: "You are in plan mode.".to_string(),
                permission: super::config::AgentPermissionConfig::builtin_plan(),
                max_steps: Some(100),
                model_id: None,
                temperature: None,
                top_p: None,
                color: None,
                hidden: false,
                source_path: None,
                is_builtin: true,
            },
            AgentConfig {
                name: "explore".to_string(),
                description: Some("Read-only codebase exploration subagent".to_string()),
                mode: AgentMode::Subagent,
                system_prompt: "You explore the codebase with read-only tools.".to_string(),
                permission: Default::default(),
                max_steps: Some(50),
                model_id: None,
                temperature: None,
                top_p: None,
                color: None,
                hidden: false,
                source_path: None,
                is_builtin: true,
            },
        ]
    }

    pub async fn load_for_workspace(workspace_root: &Path) -> AgentResult<HashMap<String, AgentConfig>> {
        let mut configs: HashMap<String, AgentConfig> = Self::builtin()
            .into_iter()
            .map(|cfg| (cfg.name.clone(), cfg))
            .collect();

        let dir = workspace_root.join(".orbitx").join("agents");
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

    async fn load_one(path: &PathBuf) -> AgentResult<AgentConfig> {
        let raw = fs::read_to_string(path)
            .await
            .map_err(AgentError::from)?;

        let (front, body) = split_frontmatter(&raw);
        let front = front.ok_or_else(|| {
            AgentError::Parse(format!(
                "Missing frontmatter in agent config {}",
                path.display()
            ))
        })?;

        let parsed = parse_frontmatter(front);
        let name = parsed
            .fields
            .get("name")
            .cloned()
            .ok_or_else(|| AgentError::Parse("Missing agent name".to_string()))?;

        let description = parsed.fields.get("description").cloned();
        let mode = parsed
            .fields
            .get("mode")
            .map(|s| parse_agent_mode(s))
            .transpose()?
            .unwrap_or(AgentMode::Primary);
        let permission = build_permission_config(&parsed)?;

        let max_steps = parsed
            .fields
            .get("max_steps")
            .and_then(|v| v.parse::<u32>().ok());
        let model_id = parsed.fields.get("model").cloned();
        let temperature = parsed
            .fields
            .get("temperature")
            .and_then(|v| v.parse::<f32>().ok());
        let top_p = parsed.fields.get("top_p").and_then(|v| v.parse::<f32>().ok());
        let hidden = parsed
            .fields
            .get("hidden")
            .map(|v| v == "true")
            .unwrap_or(false);

        Ok(AgentConfig {
            name,
            description,
            mode,
            system_prompt: body.trim().to_string(),
            permission,
            max_steps,
            model_id,
            temperature,
            top_p,
            color: parsed.fields.get("color").cloned(),
            hidden,
            source_path: Some(path.to_string_lossy().to_string()),
            is_builtin: false,
        })
    }
}
