/*!
 * RunnableTool trait & related types (agent/tools)
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::metadata::ToolMetadata;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;

/// Context for dynamic tool description generation
#[derive(Debug, Clone)]
pub struct ToolDescriptionContext {
    pub cwd: String,
}

/// Context for checking tool availability at registration time
#[derive(Debug, Clone, Default)]
pub struct ToolAvailabilityContext {
    pub has_vector_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Success(String),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolResultContent>,
    pub status: ToolResultStatus,
    #[serde(rename = "cancelReason")]
    pub cancel_reason: Option<String>,
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: Option<u64>,
    #[serde(rename = "extInfo")]
    pub ext_info: Option<Value>,
}

#[async_trait]
pub trait RunnableTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;

    /// Check if this tool is available for registration.
    /// Tools can override this to conditionally disable themselves.
    /// Default: always available.
    fn is_available(&self, _ctx: &ToolAvailabilityContext) -> bool {
        true
    }

    /// Optional dynamic description based on context
    /// Returns None to use the static description()
    fn description_with_context(&self, _context: &ToolDescriptionContext) -> Option<String> {
        None
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }
    fn tags(&self) -> Vec<String> {
        self.metadata().tags
    }

    /// Optional validation based on parameters_schema; default: no-op
    fn validate_arguments(&self, _args: &Value) -> ToolExecutorResult<()> {
        Ok(())
    }

    /// Optional lifecycle hooks
    async fn before_run(&self, _context: &TaskContext, _args: &Value) -> ToolExecutorResult<()> {
        Ok(())
    }
    async fn after_run(
        &self,
        _context: &TaskContext,
        _result: &ToolResult,
    ) -> ToolExecutorResult<()> {
        Ok(())
    }

    async fn run(&self, context: &TaskContext, args: Value) -> ToolExecutorResult<ToolResult>;

    /// Default: build ToolSchema from basic fields
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}
