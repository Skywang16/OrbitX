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

/// Per-component toggle/metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub enabled: bool,
    pub priority: u32,
}

/// Prompt configuration with default ordering and overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub default_component_order: HashMap<PromptType, Vec<PromptComponent>>,
    pub template_overrides: HashMap<String, HashMap<PromptComponent, String>>,
    pub component_config: HashMap<PromptComponent, ComponentConfig>,
}

impl Default for PromptConfig {
    fn default() -> Self {
        let mut default_component_order = HashMap::new();

        // 精简后的组件顺序（按 priority 排序）
        default_component_order.insert(
            PromptType::Agent,
            vec![
                PromptComponent::AgentRole,          // priority: 100 - 角色定义
                PromptComponent::AgentRules,         // priority: 90  - 行为规则
                PromptComponent::WorkMethodology,    // priority: 80  - 工作方法
                PromptComponent::SystemInfo,         // priority: 70  - 系统信息
                PromptComponent::TaskContext,        // priority: 60  - 任务上下文
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
            },
        );
        component_config.insert(
            PromptComponent::AgentRules,
            ComponentConfig {
                enabled: true,
                priority: 90,
            },
        );
        component_config.insert(
            PromptComponent::WorkMethodology,
            ComponentConfig {
                enabled: true,
                priority: 80,
            },
        );
        component_config.insert(
            PromptComponent::SystemInfo,
            ComponentConfig {
                enabled: true,
                priority: 70,
            },
        );
        component_config.insert(
            PromptComponent::TaskContext,
            ComponentConfig {
                enabled: true,
                priority: 60,
            },
        );
        component_config.insert(
            PromptComponent::CustomInstructions,
            ComponentConfig {
                enabled: true,
                priority: 50,
            },
        );

        Self {
            default_component_order,
            template_overrides,
            component_config,
        }
    }
}

impl PromptConfig {
    pub fn component_order(&self, prompt_type: PromptType) -> Vec<PromptComponent> {
        let mut order = self
            .default_component_order
            .get(&prompt_type)
            .cloned()
            .unwrap_or_default();

        order.retain(|component| {
            self.component_config
                .get(component)
                .map(|c| c.enabled)
                .unwrap_or(true)
        });

        order.sort_by_key(|component| {
            self.component_config
                .get(component)
                .map(|c| c.priority)
                .unwrap_or(0)
        });

        order
    }

    pub fn template_overrides_for(
        &self,
        scenario: Option<&str>,
    ) -> HashMap<PromptComponent, String> {
        let name = scenario.unwrap_or("default");
        self.template_overrides
            .get(name)
            .cloned()
            .unwrap_or_default()
    }
}
