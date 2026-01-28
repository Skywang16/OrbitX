//! Agent 配置加载器
//!
//! 从 prompts/agents/*.md 加载内置 agent，
//! 从 .orbitx/agents/*.md 加载用户自定义 agent

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::agent::agents::config::{AgentConfig, AgentMode};
use crate::agent::agents::frontmatter::{
    parse_agent_mode, parse_frontmatter, parse_tool_filter, split_frontmatter,
};
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::BuiltinPrompts;

pub struct AgentConfigLoader;

impl AgentConfigLoader {
    /// 解析 agent md 文件内容为 AgentConfig
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

        let max_steps = parsed
            .fields
            .get("max_steps")
            .and_then(|v| v.parse::<u32>().ok());

        let model_id = parsed.fields.get("model").cloned();

        let temperature = parsed
            .fields
            .get("temperature")
            .and_then(|v| v.parse::<f32>().ok());

        let top_p = parsed
            .fields
            .get("top_p")
            .and_then(|v| v.parse::<f32>().ok());

        let hidden = parsed
            .fields
            .get("hidden")
            .map(|v| v == "true")
            .unwrap_or(false);

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

    /// 加载内置 agents
    pub fn builtin() -> Vec<AgentConfig> {
        let builtin_contents = [
            ("coder", BuiltinPrompts::agent_coder()),
            ("plan", BuiltinPrompts::agent_plan()),
            ("explore", BuiltinPrompts::agent_explore()),
            ("general", BuiltinPrompts::agent_general()),
            ("research", BuiltinPrompts::agent_research()),
        ];

        builtin_contents
            .iter()
            .filter_map(|(name, content)| {
                Self::parse_agent_content(content, None, true)
                    .map_err(|e| {
                        eprintln!("Failed to parse builtin agent {}: {}", name, e);
                        e
                    })
                    .ok()
            })
            .collect()
    }

    /// 加载 workspace 和 builtin agents
    pub async fn load_for_workspace(
        workspace_root: &Path,
    ) -> AgentResult<HashMap<String, AgentConfig>> {
        // 先加载 builtin
        let mut configs: HashMap<String, AgentConfig> = Self::builtin()
            .into_iter()
            .map(|cfg| (cfg.name.clone(), cfg))
            .collect();

        // 再加载 workspace 自定义（会覆盖同名 builtin）
        let dir = workspace_root.join(".orbitx").join("agents");
        let Ok(mut entries) = fs::read_dir(&dir).await else {
            return Ok(configs);
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(cfg) = Self::parse_agent_content(
                    &content,
                    Some(path.to_string_lossy().to_string()),
                    false,
                ) {
                    configs.insert(cfg.name.clone(), cfg);
                }
            }
        }

        Ok(configs)
    }
}
