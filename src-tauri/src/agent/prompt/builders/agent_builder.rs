use std::collections::HashMap;
use std::io::Cursor;

use crate::agent::config::{PromptComponent, PromptConfig, PromptType};
use crate::agent::error::AgentResult;
use crate::agent::prompt::builders::prompt_builder::{PromptBuildOptions, PromptBuilder};
use crate::agent::prompt::components::types::ComponentContext;
use crate::agent::{Agent, Context, Task, ToolSchema};
use xmltree::{Element, XMLNode};

const TASK_NODE_STATUS_TOOL: &str = "task_node_status";

pub struct AgentPromptBuilder {
    builder: PromptBuilder,
    config: PromptConfig,
}

impl AgentPromptBuilder {
    pub fn new() -> Self {
        Self {
            builder: PromptBuilder::new(),
            config: PromptConfig::default(),
        }
    }

    pub async fn build_agent_system_prompt(
        &mut self,
        agent: Agent,
        task: Option<Task>,
        context: Option<Context>,
        tools: Vec<ToolSchema>,
        ext_sys_prompt: Option<String>,
    ) -> AgentResult<String> {
        let component_context = ComponentContext {
            agent,
            task,
            context,
            tools,
            ext_sys_prompt,
            additional_context: HashMap::new(),
        };

        let components = component_order(&self.config, PromptType::Agent);
        let template_overrides = scenario_template_overrides(&self.config, None);

        let options = PromptBuildOptions {
            components,
            template_overrides,
            skip_missing: true,
            ..Default::default()
        };

        self.builder
            .build(component_context, options)
            .await
            .map(|result| result.trim().to_string())
    }
}

fn component_order(config: &PromptConfig, prompt_type: PromptType) -> Vec<PromptComponent> {
    let mut order = config
        .default_component_order
        .get(&prompt_type)
        .cloned()
        .unwrap_or_default();

    order.retain(|component| {
        config
            .component_config
            .get(component)
            .map(|c| c.enabled)
            .unwrap_or(true)
    });

    order.sort_by_key(|component| {
        config
            .component_config
            .get(component)
            .map(|c| c.priority)
            .unwrap_or(0)
    });

    order
}

fn scenario_template_overrides(
    config: &PromptConfig,
    scenario: Option<&str>,
) -> HashMap<PromptComponent, String> {
    if let Some(name) = scenario {
        if let Some(overrides) = config.template_overrides.get(name) {
            return overrides.clone();
        }
    }

    config
        .template_overrides
        .get("default")
        .cloned()
        .unwrap_or_default()
}

pub async fn build_agent_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
    ext_sys_prompt: Option<String>,
) -> AgentResult<String> {
    let mut builder = AgentPromptBuilder::new();
    builder
        .build_agent_system_prompt(agent, task, context, tools, ext_sys_prompt)
        .await
}

fn mark_nodes_recursive(element: &mut Element, mark_nodes: bool) {
    if element.name == "node" && mark_nodes {
        element.attributes.insert("status".into(), "todo".into());
    }
    for child in element.children.iter_mut() {
        if let XMLNode::Element(child_el) = child {
            mark_nodes_recursive(child_el, mark_nodes);
        }
    }
}

fn build_agent_root_xml(agent_xml: &str, task_prompt: &str, mark_nodes: bool) -> Option<String> {
    let inner = if agent_xml.trim().is_empty() {
        "<agent />".to_string()
    } else {
        agent_xml.to_string()
    };

    let wrapped = format!("<root>{}</root>", inner);
    let mut root = Element::parse(Cursor::new(wrapped)).ok()?;

    if !task_prompt.trim().is_empty() {
        let mut instruction = Element::new("instruction");
        instruction
            .children
            .push(XMLNode::Text(task_prompt.trim().to_string()));
        root.children.insert(0, XMLNode::Element(instruction));
    }

    mark_nodes_recursive(&mut root, mark_nodes);

    let mut buffer = Vec::new();
    root.write(&mut buffer).ok()?;
    String::from_utf8(buffer).ok()
}

pub async fn build_agent_user_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    let _ = agent; // currently unused but reserved for future parity with frontend

    let has_task_node_status_tool = tools.iter().any(|tool| tool.name == TASK_NODE_STATUS_TOOL);

    let agent_xml = task
        .as_ref()
        .and_then(|t| t.xml.clone())
        .unwrap_or_else(|| "<agent />".to_string());

    let task_prompt = context
        .as_ref()
        .and_then(|ctx| ctx.additional_context.get("taskPrompt"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    let built = build_agent_root_xml(&agent_xml, &task_prompt, has_task_node_status_tool)
        .unwrap_or_else(|| format!("<root>{}</root>", agent_xml));

    Ok(built)
}
