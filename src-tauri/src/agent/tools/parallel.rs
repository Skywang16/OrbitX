/*!
 * 并行工具执行器
 *
 * 根据 ToolCategory 自动判断并行性：
 * - FileRead/CodeAnalysis/FileSystem/Network: 可并行
 * - FileWrite/Execution/Terminal: 必须串行
 */

use futures::future::join_all;
use serde_json::Value;

use super::metadata::ExecutionMode;
use super::registry::ToolRegistry;
use super::ToolResult;
use crate::agent::core::context::TaskContext;

/// 最大并行数，防止资源耗尽
const MAX_CONCURRENCY: usize = 8;

/// 工具调用请求
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub params: Value,
}

/// 工具调用结果
pub struct ToolCallResult {
    pub id: String,
    pub name: String,
    pub result: ToolResult,
}

/// 执行一批工具调用，自动并行化只读操作
///
/// 分组策略：连续的可并行工具放入同一组并发执行，遇到串行工具则开新组
pub async fn execute_batch(
    registry: &ToolRegistry,
    context: &TaskContext,
    calls: Vec<ToolCall>,
) -> Vec<ToolCallResult> {
    if calls.is_empty() {
        return vec![];
    }

    // 单个调用直接执行，无需分组
    if calls.len() == 1 {
        let call = calls.into_iter().next().unwrap();
        let result = registry
            .execute_tool(&call.name, context, call.params)
            .await;
        return vec![ToolCallResult {
            id: call.id,
            name: call.name,
            result,
        }];
    }

    // 分组并执行
    let groups = group_by_mode(registry, &calls).await;
    let mut results = Vec::with_capacity(calls.len());

    for group in groups {
        let group_results = execute_group(registry, context, group).await;
        results.extend(group_results);
    }

    results
}

/// 执行组：(起始索引, 是否并行, 调用列表)
type Group<'a> = (usize, bool, Vec<&'a ToolCall>);

/// 根据执行模式分组
async fn group_by_mode<'a>(registry: &ToolRegistry, calls: &'a [ToolCall]) -> Vec<Group<'a>> {
    let mut groups: Vec<Group<'a>> = Vec::new();

    for (idx, call) in calls.iter().enumerate() {
        let is_parallel = get_execution_mode(registry, &call.name).await == ExecutionMode::Parallel;

        match groups.last_mut() {
            // 首个调用，开新组
            None => groups.push((idx, is_parallel, vec![call])),
            // 当前和上一组都是并行，合并
            Some((_, true, ref mut group_calls)) if is_parallel => {
                group_calls.push(call);
            }
            // 否则开新组
            _ => groups.push((idx, is_parallel, vec![call])),
        }
    }

    groups
}

/// 获取工具执行模式
#[inline]
async fn get_execution_mode(registry: &ToolRegistry, name: &str) -> ExecutionMode {
    registry
        .get_tool_metadata(name)
        .await
        .map(|m| m.category.execution_mode())
        .unwrap_or(ExecutionMode::Sequential)
}

/// 执行单个组
async fn execute_group(
    registry: &ToolRegistry,
    context: &TaskContext,
    (start_idx, is_parallel, calls): Group<'_>,
) -> Vec<ToolCallResult> {
    if is_parallel && calls.len() > 1 {
        execute_parallel(registry, context, start_idx, calls).await
    } else {
        execute_sequential(registry, context, start_idx, calls).await
    }
}

/// 并行执行（限制并发数）
async fn execute_parallel(
    registry: &ToolRegistry,
    context: &TaskContext,
    _start_idx: usize,
    calls: Vec<&ToolCall>,
) -> Vec<ToolCallResult> {
    tracing::info!("[execute_parallel] Starting {} parallel calls", calls.len());
    // 分批执行，每批最多 MAX_CONCURRENCY 个
    let mut results = Vec::with_capacity(calls.len());

    for (_chunk_idx, chunk) in calls.chunks(MAX_CONCURRENCY).enumerate() {
        let futures = chunk.iter().map(|call| async {
            let result = registry
                .execute_tool(&call.name, context, call.params.clone())
                .await;
            ToolCallResult {
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

/// 串行执行
async fn execute_sequential(
    registry: &ToolRegistry,
    context: &TaskContext,
    _start_idx: usize,
    calls: Vec<&ToolCall>,
) -> Vec<ToolCallResult> {
    let mut results = Vec::with_capacity(calls.len());

    for call in calls.iter() {
        let result = registry
            .execute_tool(&call.name, context, call.params.clone())
            .await;
        results.push(ToolCallResult {
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
    }
}
