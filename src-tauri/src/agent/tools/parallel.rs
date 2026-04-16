/*!
 * Parallel tool executor
 *
 * Automatically determines parallelism based on ToolCategory:
 * - FileRead/CodeAnalysis/FileSystem/Network: can be parallelized
 * - FileWrite/Execution/Terminal: must be sequential
 */

use futures::future::join_all;
use serde_json::Value;

use super::metadata::ExecutionMode;
use super::registry::ToolRegistry;
use super::ToolResult;
use crate::agent::core::context::AgentRunContext;

/// Maximum concurrency to prevent resource exhaustion
const MAX_CONCURRENCY: usize = 8;

/// Tool call request
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub params: Value,
}

/// Result from a single tool execution within a batch
pub struct BatchToolResult {
    pub id: String,
    pub name: String,
    pub result: ToolResult,
}

/// Execute a batch of tool calls, automatically parallelizing read-only operations
///
/// Grouping strategy: consecutive parallelizable tools are grouped together for concurrent execution; sequential tools start a new group
pub async fn execute_batch(
    registry: &ToolRegistry,
    context: &AgentRunContext,
    calls: Vec<ToolCall>,
) -> Vec<BatchToolResult> {
    if calls.is_empty() {
        return vec![];
    }

    // Single call executes directly, no grouping needed
    if calls.len() == 1 {
        let mut iter = calls.into_iter();
        let Some(call) = iter.next() else {
            return vec![];
        };
        let result = registry
            .execute_tool(&call.name, context, call.params)
            .await;
        return vec![BatchToolResult {
            id: call.id,
            name: call.name,
            result,
        }];
    }

    // Group and execute
    let groups = group_by_mode(registry, &calls).await;
    let mut results = Vec::with_capacity(calls.len());

    for group in groups {
        let group_results = execute_group(registry, context, group).await;
        results.extend(group_results);
    }

    results
}

/// Execution group: (start index, is parallel, call list)
type Group<'a> = (usize, bool, Vec<&'a ToolCall>);

/// Group by execution mode
async fn group_by_mode<'a>(registry: &ToolRegistry, calls: &'a [ToolCall]) -> Vec<Group<'a>> {
    let mut groups: Vec<Group<'a>> = Vec::new();

    for (idx, call) in calls.iter().enumerate() {
        let is_parallel = get_execution_mode(registry, &call.name).await == ExecutionMode::Parallel;

        match groups.last_mut() {
            // First call, start new group
            None => groups.push((idx, is_parallel, vec![call])),
            // Current and previous group are both parallel, merge
            Some((_, true, ref mut group_calls)) if is_parallel => {
                group_calls.push(call);
            }
            // Otherwise start new group
            _ => groups.push((idx, is_parallel, vec![call])),
        }
    }

    groups
}

/// Get tool execution mode
#[inline]
async fn get_execution_mode(registry: &ToolRegistry, name: &str) -> ExecutionMode {
    registry
        .get_tool_metadata(name)
        .await
        .map(|m| {
            m.execution_mode
                .unwrap_or_else(|| m.category.execution_mode())
        })
        .unwrap_or(ExecutionMode::Sequential)
}

/// Execute a single group
async fn execute_group(
    registry: &ToolRegistry,
    context: &AgentRunContext,
    (start_idx, is_parallel, calls): Group<'_>,
) -> Vec<BatchToolResult> {
    if is_parallel && calls.len() > 1 {
        execute_parallel(registry, context, start_idx, calls).await
    } else {
        execute_sequential(registry, context, start_idx, calls).await
    }
}

/// Parallel execution (with concurrency limit)
async fn execute_parallel(
    registry: &ToolRegistry,
    context: &AgentRunContext,
    _start_idx: usize,
    calls: Vec<&ToolCall>,
) -> Vec<BatchToolResult> {
    tracing::info!("[execute_parallel] Starting {} parallel calls", calls.len());
    // Execute in batches, at most MAX_CONCURRENCY per batch
    let mut results = Vec::with_capacity(calls.len());

    for chunk in calls.chunks(MAX_CONCURRENCY) {
        let futures = chunk.iter().map(|call| async {
            let result = registry
                .execute_tool(&call.name, context, call.params.clone())
                .await;
            BatchToolResult {
                id: call.id.clone(),
                name: call.name.clone(),
                result,
            }
        });

        let chunk_results = join_all(futures).await;
        results.extend(chunk_results);
    }

    results
}

/// Sequential execution
async fn execute_sequential(
    registry: &ToolRegistry,
    context: &AgentRunContext,
    _start_idx: usize,
    calls: Vec<&ToolCall>,
) -> Vec<BatchToolResult> {
    let mut results = Vec::with_capacity(calls.len());

    for call in calls.iter() {
        let result = registry
            .execute_tool(&call.name, context, call.params.clone())
            .await;
        results.push(BatchToolResult {
            id: call.id.clone(),
            name: call.name.clone(),
            result,
        });
    }

    tracing::info!("[execute_sequential] All sequential calls completed");
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode() {
        use super::super::metadata::ToolCategory;

        assert_eq!(
            ToolCategory::FileRead.execution_mode(),
            ExecutionMode::Parallel
        );
        assert_eq!(
            ToolCategory::FileWrite.execution_mode(),
            ExecutionMode::Sequential
        );
        assert_eq!(
            ToolCategory::Network.execution_mode(),
            ExecutionMode::Parallel
        );
        assert_eq!(
            ToolCategory::Execution.execution_mode(),
            ExecutionMode::Sequential
        );
        assert_eq!(
            ToolCategory::Delegation.execution_mode(),
            ExecutionMode::Sequential
        );
    }
}
