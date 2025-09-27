use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt components align with eko-core definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum PromptComponent {
    // Agent level
    AgentRole,
    AgentDescription,
    AgentCapabilities,

    // System and context
    SystemInfo,
    DateTime,
    Platform,

    // Tools and interaction
    ToolsDescription,

    // Task specific
    TaskContext,
    TaskNodes,
    TaskExamples,

    // Planning
    PlanningGuidelines,
    PlanningExamples,
    OutputFormat,

    // Dialogue
    DialogueCapabilities,
    DialogueGuidelines,

    // Extras
    WorkspaceSnapshot,
    CustomInstructions,
    AdditionalContext,
    AgentRules,
    WorkMethodology,
}

/// Prompt channels: agent / dialogue / planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PromptType {
    Agent,
    Dialogue,
    Planning,
}

/// Conditional rule for component evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalRule {
    pub condition: String,
    pub action: String,
    pub params: Option<serde_json::Value>,
}

/// Per-component toggle/metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub enabled: bool,
    pub priority: u32,
    pub dependencies: Vec<PromptComponent>,
    pub conditional_rules: Vec<ConditionalRule>,
}

/// Prompt configuration with default ordering and overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub default_component_order: HashMap<PromptType, Vec<PromptComponent>>,
    pub template_overrides: HashMap<String, HashMap<PromptComponent, String>>,
    pub enabled_features: Vec<String>,
    pub component_config: HashMap<PromptComponent, ComponentConfig>,
    pub variants: HashMap<String, PromptVariant>,
}

/// A named prompt variant referencing a subset of components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariant {
    pub prompt_type: PromptType,
    pub components: Vec<PromptComponent>,
    pub template: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        let mut default_component_order = HashMap::new();

        default_component_order.insert(
            PromptType::Agent,
            vec![
                PromptComponent::AgentRole,
                PromptComponent::AgentCapabilities,
                PromptComponent::AgentRules,
                PromptComponent::WorkMethodology,
                PromptComponent::SystemInfo,
                PromptComponent::TaskContext,
                PromptComponent::ToolsDescription,
                PromptComponent::TaskNodes,
                PromptComponent::CustomInstructions,
                PromptComponent::DateTime,
            ],
        );

        default_component_order.insert(
            PromptType::Dialogue,
            vec![
                PromptComponent::AgentRole,
                PromptComponent::DialogueCapabilities,
                PromptComponent::DialogueGuidelines,
                PromptComponent::CustomInstructions,
                PromptComponent::DateTime,
            ],
        );

        default_component_order.insert(
            PromptType::Planning,
            vec![
                PromptComponent::AgentRole,
                PromptComponent::PlanningGuidelines,
                PromptComponent::AgentCapabilities,
                PromptComponent::OutputFormat,
                PromptComponent::PlanningExamples,
                PromptComponent::DateTime,
            ],
        );

        let mut template_overrides = HashMap::new();
        template_overrides.insert(
            "default".to_string(),
            HashMap::from([(
                PromptComponent::AgentRole,
                "You are {name}, a terminal-focused AI assistant.".to_string(),
            )]),
        );

        template_overrides.insert(
            "development".to_string(),
            HashMap::from([(
                PromptComponent::AgentRole,
                "You are {name}, a development-focused AI assistant.".to_string(),
            )]),
        );

        let mut component_config = HashMap::new();
        component_config.insert(
            PromptComponent::AgentRole,
            ComponentConfig {
                enabled: true,
                priority: 100,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::AgentDescription,
            ComponentConfig {
                enabled: true,
                priority: 90,
                dependencies: vec![PromptComponent::AgentRole],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::SystemInfo,
            ComponentConfig {
                enabled: true,
                priority: 80,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::AgentCapabilities,
            ComponentConfig {
                enabled: true,
                priority: 70,
                dependencies: vec![PromptComponent::ToolsDescription],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::TaskContext,
            ComponentConfig {
                enabled: true,
                priority: 60,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::TaskNodes,
            ComponentConfig {
                enabled: true,
                priority: 50,
                dependencies: vec![PromptComponent::TaskContext],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::CustomInstructions,
            ComponentConfig {
                enabled: true,
                priority: 40,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::DateTime,
            ComponentConfig {
                enabled: true,
                priority: 30,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::WorkspaceSnapshot,
            ComponentConfig {
                enabled: false,
                priority: 20,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::ToolsDescription,
            ComponentConfig {
                enabled: true,
                priority: 50,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::DialogueCapabilities,
            ComponentConfig {
                enabled: true,
                priority: 50,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::DialogueGuidelines,
            ComponentConfig {
                enabled: true,
                priority: 40,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::PlanningGuidelines,
            ComponentConfig {
                enabled: true,
                priority: 60,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::PlanningExamples,
            ComponentConfig {
                enabled: true,
                priority: 50,
                dependencies: vec![PromptComponent::PlanningGuidelines],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::OutputFormat,
            ComponentConfig {
                enabled: true,
                priority: 40,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::Platform,
            ComponentConfig {
                enabled: true,
                priority: 30,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::TaskExamples,
            ComponentConfig {
                enabled: false,
                priority: 30,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::AdditionalContext,
            ComponentConfig {
                enabled: false,
                priority: 20,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::AgentRules,
            ComponentConfig {
                enabled: true,
                priority: 90,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );
        component_config.insert(
            PromptComponent::WorkMethodology,
            ComponentConfig {
                enabled: true,
                priority: 80,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );

        let mut variants = HashMap::new();
        variants.insert(
            "minimal_agent".to_string(),
            PromptVariant {
                prompt_type: PromptType::Agent,
                components: vec![
                    PromptComponent::AgentRole,
                    PromptComponent::AgentDescription,
                    PromptComponent::DateTime,
                ],
                template: "{AgentRole}\n{AgentDescription}\n{DateTime}".to_string(),
            },
        );
        variants.insert(
            "minimal_dialogue".to_string(),
            PromptVariant {
                prompt_type: PromptType::Dialogue,
                components: vec![
                    PromptComponent::AgentRole,
                    PromptComponent::DialogueCapabilities,
                    PromptComponent::DateTime,
                ],
                template: "{AgentRole}\n{DialogueCapabilities}\n{DateTime}".to_string(),
            },
        );

        Self {
            default_component_order,
            template_overrides,
            enabled_features: vec![],
            component_config,
            variants,
        }
    }
}
