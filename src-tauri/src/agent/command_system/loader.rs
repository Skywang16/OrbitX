use std::collections::HashMap;
use std::sync::LazyLock;

use super::types::{CommandConfig, CommandRenderResult, CommandSummary};

/// Built-in command templates, compiled into the binary.
static BUILTIN_COMMANDS: LazyLock<HashMap<String, CommandConfig>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    let builtins: &[(&str, &str)] = &[
        (
            "code-review",
            include_str!("../../../prompts/commands/code-review.md"),
        ),
        (
            "skill-creator",
            include_str!("../../../prompts/commands/skill-creator.md"),
        ),
        (
            "plan-mode",
            include_str!("../../../prompts/commands/plan-mode.md"),
        ),
        (
            "orchestrate-mode",
            include_str!("../../../prompts/commands/orchestrate-mode.md"),
        ),
        (
            "skill-installer",
            include_str!("../../../prompts/commands/skill-installer.md"),
        ),
    ];
    for (name, template) in builtins {
        m.insert(
            name.to_string(),
            CommandConfig {
                name: name.to_string(),
                description: None,
                agent: Some(match *name {
                    "plan-mode" => "plan".to_string(),
                    "orchestrate-mode" => "orchestrate".to_string(),
                    _ => "coder".to_string(),
                }),
                model: None,
                delegated: false,
                template: template.trim().to_string(),
            },
        );
    }
    m
});

pub struct CommandConfigLoader;

impl CommandConfigLoader {
    pub fn get(command_id: &str) -> Option<&'static CommandConfig> {
        BUILTIN_COMMANDS.get(command_id)
    }

    pub fn all() -> &'static HashMap<String, CommandConfig> {
        &BUILTIN_COMMANDS
    }

    pub fn render(cfg: &CommandConfig, input: &str) -> CommandRenderResult {
        let prompt = cfg.template.replace("{{input}}", input);
        CommandRenderResult {
            name: cfg.name.clone(),
            agent: cfg.agent.clone(),
            model: cfg.model.clone(),
            delegated: cfg.delegated,
            prompt,
        }
    }

    pub fn summarize(cfg: &CommandConfig) -> CommandSummary {
        CommandSummary {
            name: cfg.name.clone(),
            description: cfg.description.clone(),
            agent: cfg.agent.clone(),
            model: cfg.model.clone(),
            delegated: cfg.delegated,
        }
    }
}
