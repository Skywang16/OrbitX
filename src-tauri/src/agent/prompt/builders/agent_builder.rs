use std::collections::HashMap;
use std::io::Cursor;

use crate::agent::config::{PromptConfig, PromptType};
use crate::agent::error::AgentResult;
use crate::agent::prompt::builders::prompt_builder::{PromptBuildOptions, PromptBuilder};
use crate::agent::prompt::components::types::ComponentContext;
use crate::agent::{Agent, Context, Task, ToolSchema};
use xmltree::{Element, XMLNode};

const TASK_NODE_STATUS_TOOL: &str = "task_node_status";

pub async fn build_agent_system_prompt(
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

    let config = PromptConfig::default();
    let options = PromptBuildOptions {
        components: config.component_order(PromptType::Agent),
        template_overrides: config.template_overrides_for(None),
        ..Default::default()
    };

    let mut builder = PromptBuilder::new();
    builder
        .build(component_context, options)
        .await
        .map(|result| result.trim().to_string())
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

    let wrapped = format!("<root>{inner}</root>");
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
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
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
        .unwrap_or_else(|| format!("<root>{agent_xml}</root>"));

    Ok(built)
}
