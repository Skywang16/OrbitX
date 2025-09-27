use std::sync::{Arc, Mutex};

use crate::agent::tools::ToolResult;
use crate::llm::types::{LLMRequest, LLMToolCall};

/// Event emitted when the tool chain updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainEventType {
    Update,
}

#[derive(Debug, Clone)]
pub struct ChainEvent {
    pub event_type: ChainEventType,
    pub target: ToolChain,
}

pub type ChainListener = Arc<dyn Fn(&Chain, &ChainEvent) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub struct ToolChain {
    pub tool_name: String,
    pub tool_call_id: String,
    pub request: LLMRequest,
    pub params: Option<serde_json::Value>,
    pub tool_result: Option<ToolResult>,
}

impl ToolChain {
    pub fn new(tool_call: &LLMToolCall, request: &LLMRequest) -> Self {
        Self {
            tool_name: tool_call.name.clone(),
            tool_call_id: tool_call.id.clone(),
            request: request.clone(),
            params: None,
            tool_result: None,
        }
    }

    pub fn update_params(&mut self, params: serde_json::Value) {
        self.params = Some(params);
    }

    pub fn update_tool_result(&mut self, result: ToolResult) {
        self.tool_result = Some(result);
    }
}

#[derive(Clone)]
pub struct Chain {
    pub task_prompt: String,
    pub plan_request: Option<LLMRequest>,
    pub plan_result: Option<String>,
    pub tools: Vec<ToolChain>,
    listeners: Arc<Mutex<Vec<ChainListener>>>,
}

impl Chain {
    pub fn new(task_prompt: impl Into<String>) -> Self {
        Self {
            task_prompt: task_prompt.into(),
            plan_request: None,
            plan_result: None,
            tools: Vec::new(),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push(&mut self, tool: ToolChain) {
        self.tools.push(tool.clone());
        self.notify(ChainEvent {
            event_type: ChainEventType::Update,
            target: tool,
        });
    }

    pub fn update_tool_result(&mut self, tool_call_id: &str, result: ToolResult) {
        let mut updated: Option<ToolChain> = None;
        if let Some(chain) = self
            .tools
            .iter_mut()
            .find(|t| t.tool_call_id == tool_call_id)
        {
            chain.update_tool_result(result.clone());
            updated = Some(chain.clone());
        }

        if let Some(target) = updated {
            self.notify(ChainEvent {
                event_type: ChainEventType::Update,
                target,
            });
        }
    }

    pub fn add_listener(&self, listener: ChainListener) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.push(listener);
        }
    }

    pub fn remove_listener(&self, listener_ptr: *const ()) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.retain(|listener| Arc::as_ptr(listener) as *const () != listener_ptr);
        }
    }

    pub fn set_task_prompt(&mut self, prompt: impl Into<String>) {
        self.task_prompt = prompt.into();
    }

    fn notify(&self, event: ChainEvent) {
        if let Ok(listeners) = self.listeners.lock() {
            for listener in listeners.iter() {
                listener(self, &event);
            }
        }
    }
}
