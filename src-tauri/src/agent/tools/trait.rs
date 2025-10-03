/*!
 * RunnableTool trait & related types (agent/tools)
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::ToolExecutorResult;
use crate::agent::state::context::TaskContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolResultContent {
    Text {
        text: String,
    },
    Json {
        data: Value,
    },
    CommandOutput {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    File {
        path: String,
    },
    Image {
        base64: String,
        format: String,
    },
    Error {
        message: String,
        details: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolResultContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: Option<u64>,
    #[serde(rename = "extInfo")]
    pub ext_info: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPermission {
    ReadOnly,
    FileSystem,
    SystemCommand,
    Network,
}

#[async_trait]
pub trait RunnableTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::ReadOnly]
    }
    fn tags(&self) -> Vec<String> {
        vec![]
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

    /// Default: ensure all required permissions are present in granted list
    fn check_permissions(&self, granted: &Vec<ToolPermission>) -> bool {
        let required = self.required_permissions();
        required.into_iter().all(|r| granted.contains(&r))
    }
}
