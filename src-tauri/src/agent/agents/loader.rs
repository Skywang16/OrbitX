//! Agent configuration loader
//!
//! Load builtin agents from prompts/agents/*.md,
//! load user custom agents from .orbitx/agents/*.md

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::agent::agents::config::{AgentConfig, AgentMode};
use crate::agent::agents::frontmatter::{
    parse_agent_mode, parse_frontmatter, parse_task_permissions, parse_tool_filter,
    split_frontmatter,
};
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::BuiltinPrompts;
use tracing::warn;

pub struct AgentConfigLoader;

impl AgentConfigLoader {
    fn parse_optional_u32_field(
        parsed: &crate::agent::agents::frontmatter::ParsedFrontmatter,
        key: &str,
    ) -> AgentResult<Option<u32>> {
        match parsed.fields.get(key) {
            Some(raw) => raw
                .parse::<u32>()
                .map(Some)
                .map_err(|err| AgentError::Parse(format!("Invalid {key} value '{raw}': {err}"))),
            None => Ok(None),
        }
    }

    fn parse_optional_f32_field(
        parsed: &crate::agent::agents::frontmatter::ParsedFrontmatter,
        key: &str,
    ) -> AgentResult<Option<f32>> {
        match parsed.fields.get(key) {
            Some(raw) => raw
                .parse::<f32>()
                .map(Some)
                .map_err(|err| AgentError::Parse(format!("Invalid {key} value '{raw}': {err}"))),
            None => Ok(None),
        }
    }

    fn parse_bool_field(
        parsed: &crate::agent::agents::frontmatter::ParsedFrontmatter,
        key: &str,
        default: bool,
    ) -> AgentResult<bool> {
        match parsed.fields.get(key) {
            Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                other => Err(AgentError::Parse(format!(
                    "Invalid {key} value '{other}', expected true or false"
                ))),
            },
            None => Ok(default),
        }
    }

    /// Parse agent md file content into AgentConfig
    fn parse_agent_content(
        content: &str,
        source_path: Option<String>,
        is_builtin: bool,
    ) -> AgentResult<AgentConfig> {
        let (front, body) = split_frontmatter(content);

        let front = front
            .ok_or_else(|| AgentError::Parse("Missing frontmatter in agent config".to_string()))?;

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

        let tool_filter = parse_tool_filter(front)?;
        let task_permissions = parse_task_permissions(front)?;

        let max_steps = Self::parse_optional_u32_field(&parsed, "max_steps")?;

        let model_id = parsed.fields.get("model").cloned();

        let temperature = Self::parse_optional_f32_field(&parsed, "temperature")?;

        let top_p = Self::parse_optional_f32_field(&parsed, "top_p")?;

        let hidden = Self::parse_bool_field(&parsed, "hidden", false)?;

        let skills = parsed
            .fields
            .get("skills")
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(AgentConfig {
            name,
            description,
            mode,
            system_prompt: body.trim().to_string(),
            tool_filter,
            skills,
            task_permissions,
            max_steps,
            model_id,
            temperature,
            top_p,
            color: parsed.fields.get("color").cloned(),
            hidden,
            source_path,
            is_builtin,
        })
    }

    /// Load builtin agents
    pub fn builtin() -> Vec<AgentConfig> {
        let builtin_contents = [
            ("coder", BuiltinPrompts::agent_coder()),
            ("plan", BuiltinPrompts::agent_plan()),
            ("explore", BuiltinPrompts::agent_explore()),
            ("orchestrate", BuiltinPrompts::agent_orchestrate()),
            ("general", BuiltinPrompts::agent_general()),
            ("bulk_edit", BuiltinPrompts::agent_bulk_edit()),
            ("research", BuiltinPrompts::agent_research()),
        ];

        builtin_contents
            .iter()
            .filter_map(
                |(name, content)| match Self::parse_agent_content(content, None, true) {
                    Ok(config) => Some(config),
                    Err(err) => {
                        warn!("Failed to parse builtin agent {}: {}", name, err);
                        None
                    }
                },
            )
            .collect()
    }

    /// Load workspace and builtin agents
    pub async fn load_for_workspace(
        workspace_root: &Path,
    ) -> AgentResult<HashMap<String, AgentConfig>> {
        // First load builtin
        let mut configs: HashMap<String, AgentConfig> = Self::builtin()
            .into_iter()
            .map(|cfg| (cfg.name.clone(), cfg))
            .collect();

        // Then load workspace custom (will override builtin with same name)
        let dir = workspace_root.join(".orbitx").join("agents");
        let mut entries = match fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(configs),
            Err(err) => {
                return Err(AgentError::Io(std::io::Error::other(format!(
                    "Failed to read workspace agent directory '{}': {}",
                    dir.display(),
                    err
                ))));
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(content) => content,
                Err(err) => {
                    warn!(
                        "Failed to read workspace agent '{}': {}",
                        path.display(),
                        err
                    );
                    continue;
                }
            };

            match Self::parse_agent_content(
                &content,
                Some(path.to_string_lossy().to_string()),
                false,
            ) {
                Ok(config) => {
                    configs.insert(config.name.clone(), config);
                }
                Err(err) => {
                    warn!(
                        "Failed to parse workspace agent '{}': {}",
                        path.display(),
                        err
                    );
                }
            }
        }

        Ok(configs)
    }
}
