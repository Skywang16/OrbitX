use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt components definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum PromptComponent {
    // Agent level
    AgentRole,
    AgentRules,
    WorkMethodology,

    // System and context
    SystemInfo,

    // Task specific
    TaskContext,

    // Extras
    CustomInstructions,
}

/// Prompt type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PromptType {
    Agent,
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

        // 精简后的组件顺序（按 priority 排序）
        default_component_order.insert(
            PromptType::Agent,
            vec![
                PromptComponent::AgentRole,        // priority: 100 - 角色定义
                PromptComponent::AgentRules,       // priority: 90  - 行为规则
                PromptComponent::WorkMethodology,  // priority: 80  - 工作方法
                PromptComponent::SystemInfo,       // priority: 70  - 系统信息
                PromptComponent::TaskContext,      // priority: 60  - 任务上下文
                PromptComponent::CustomInstructions, // priority: 50 - 用户自定义指令
            ],
        );

        let mut template_overrides = HashMap::new();
        template_overrides.insert("default".to_string(), HashMap::new());

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
        component_config.insert(
            PromptComponent::SystemInfo,
            ComponentConfig {
                enabled: true,
                priority: 70,
                dependencies: vec![],
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
            PromptComponent::CustomInstructions,
            ComponentConfig {
                enabled: true,
                priority: 50,
                dependencies: vec![],
                conditional_rules: vec![],
            },
        );

        Self {
            default_component_order,
            template_overrides,
            enabled_features: vec![],
            component_config,
            variants: HashMap::new(),
        }
    }
}
