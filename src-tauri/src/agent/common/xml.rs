use std::io::Cursor;

use crate::agent::error::{AgentError, AgentResult};
use crate::agent::types::{PlannedTask, PlannedTaskNode, TaskDetail, TaskDetailStatus, TaskNode};
use xmltree::{Element, XMLNode};

fn ensure_root(xml: &str) -> String {
    let trimmed = xml.trim();
    if trimmed.starts_with("<root") {
        trimmed.to_string()
    } else {
        format!("<root>{}</root>", trimmed)
    }
}

pub fn parse_task_detail(task_id: &str, xml: &str, done: bool) -> AgentResult<TaskDetail> {
    let wrapped = ensure_root(xml);
    let root = Element::parse(Cursor::new(wrapped.as_bytes()))?;

    if root.name != "root" {
        return Err(AgentError::XmlParse(
            "Task XML must have <root> element".to_string(),
        ));
    }

    let name_text = root
        .get_child("name")
        .and_then(|el| el.get_text())
        .map(|v| v.to_string())
        .unwrap_or_default();
    let thought_text = root
        .get_child("thought")
        .and_then(|el| el.get_text())
        .map(|v| v.to_string())
        .unwrap_or_default();

    let (description, nodes, agent_xml) = if let Some(agent_el) = root.get_child("agent") {
        let description = agent_el
            .get_child("task")
            .and_then(|el| el.get_text())
            .map(|v| v.to_string())
            .unwrap_or_else(|| name_text.clone());

        let nodes = collect_task_nodes(agent_el.get_child("nodes"));

        let mut buf = Vec::new();
        agent_el.write(Cursor::new(&mut buf))?;
        let agent_xml = String::from_utf8(buf).unwrap_or_default();
        (description, nodes, agent_xml)
    } else {
        (name_text.clone(), Vec::new(), xml.to_string())
    };

    Ok(TaskDetail {
        task_id: task_id.to_string(),
        name: if name_text.is_empty() {
            description.clone()
        } else {
            name_text
        },
        thought: thought_text,
        description,
        nodes,
        status: if done {
            TaskDetailStatus::Done
        } else {
            TaskDetailStatus::Init
        },
        xml: agent_xml,
        modified: false,
        task_prompt: None,
    })
}

fn collect_task_nodes(nodes_el: Option<&Element>) -> Vec<TaskNode> {
    let mut nodes = Vec::new();
    if let Some(el) = nodes_el {
        for child in &el.children {
            if let XMLNode::Element(node_el) = child {
                if node_el.name == "node" {
                    let text = node_el
                        .get_text()
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    nodes.push(TaskNode { text });
                }
            }
        }
    }
    nodes
}

pub fn parse_task_tree(xml: &str) -> AgentResult<PlannedTask> {
    let wrapped = ensure_root(xml);
    let root = Element::parse(Cursor::new(wrapped.as_bytes()))?;
    if root.name != "root" {
        return Err(AgentError::XmlParse(
            "Tree XML must have <root> element".to_string(),
        ));
    }

    Ok(parse_planned_task(&root))
}

fn parse_planned_task(element: &Element) -> PlannedTask {
    let mut planned = PlannedTask::default();
    planned.name = element
        .get_child("name")
        .and_then(|el| el.get_text())
        .map(|v| v.to_string());
    planned.thought = element
        .get_child("thought")
        .and_then(|el| el.get_text())
        .map(|v| v.to_string());
    planned.description = element
        .get_child("task")
        .and_then(|el| el.get_text())
        .map(|v| v.to_string());

    planned.nodes = element.get_child("nodes").map(|nodes_el| {
        nodes_el
            .children
            .iter()
            .filter_map(|child| match child {
                XMLNode::Element(el) if el.name == "node" => {
                    el.get_text().map(|v| PlannedTaskNode {
                        text: v.to_string(),
                    })
                }
                _ => None,
            })
            .collect()
    });

    if let Some(subtasks_el) = element.get_child("subtasks") {
        let subtasks = subtasks_el
            .children
            .iter()
            .filter_map(|child| match child {
                XMLNode::Element(el) if el.name == "task" => Some(parse_planned_task(el)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !subtasks.is_empty() {
            planned.subtasks = Some(subtasks);
        }
    }

    planned
}

pub fn build_agent_xml_from_planned(planned: &PlannedTask) -> AgentResult<String> {
    let mut agent_el = Element::new("agent");

    if let Some(description) = planned.description.as_ref().or(planned.name.as_ref()) {
        let mut task_el = Element::new("task");
        task_el
            .children
            .push(XMLNode::Text(description.to_string()));
        agent_el.children.push(XMLNode::Element(task_el));
    }

    if let Some(nodes) = &planned.nodes {
        if !nodes.is_empty() {
            let mut nodes_el = Element::new("nodes");
            for node in nodes {
                let mut node_el = Element::new("node");
                node_el.children.push(XMLNode::Text(node.text.to_string()));
                nodes_el.children.push(XMLNode::Element(node_el));
            }
            agent_el.children.push(XMLNode::Element(nodes_el));
        }
    }

    let mut buf = Vec::new();
    agent_el.write(Cursor::new(&mut buf))?;
    String::from_utf8(buf).map_err(AgentError::Utf8)
}
