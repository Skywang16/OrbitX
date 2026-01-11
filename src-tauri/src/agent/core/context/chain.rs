use crate::agent::tools::ToolResult;

/// 工具调用链条记录
#[derive(Debug, Clone)]
pub struct ToolChain {
    pub tool_name: String,
    pub tool_call_id: String,
    pub tool_result: Option<ToolResult>,
}

impl ToolChain {
    pub fn new(tool_name: &str, tool_call_id: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            tool_call_id: tool_call_id.to_string(),
            tool_result: None,
        }
    }

    pub fn update_tool_result(&mut self, result: ToolResult) {
        self.tool_result = Some(result);
    }
}

/// 工具执行链 - 追踪一个任务中所有工具调用
#[derive(Clone, Default)]
pub struct Chain {
    pub tools: Vec<ToolChain>,
}

impl Chain {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn update_tool_result(&mut self, tool_call_id: &str, result: ToolResult) {
        if let Some(chain) = self
            .tools
            .iter_mut()
            .find(|t| t.tool_call_id == tool_call_id)
        {
            chain.update_tool_result(result);
        }
    }
}
