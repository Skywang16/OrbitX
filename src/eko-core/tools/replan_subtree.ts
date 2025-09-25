import { JSONSchema7 } from '../types'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import { TreePlanner } from '../core/plan-tree'
import { buildAgentXmlFromPlanned, parseTask } from '../common/xml'

export const TOOL_NAME = 'replan_subtree'

/**
 * 重新规划一颗子树：
 * - 对指定 targetTaskId 生成新的树计划
 * - 删除旧的子树（保留根节点本身）
 * - 用新的 planned.subtasks 递归生成子任务
 */
export default class ReplanSubtreeTool implements Tool {
  readonly name = TOOL_NAME
  readonly description = 'Re-plan a subtree starting at targetTaskId (preserving the root, recreating its children).'
  readonly parameters: JSONSchema7 = {
    type: 'object',
    properties: {
      targetTaskId: { type: 'string', description: 'The taskId of subtree root to replan.' },
      prompt: {
        type: 'string',
        description: 'Optional new instruction to guide planning. Defaults to current task description.',
      },
    },
    required: ['targetTaskId'],
  }
  readonly planDescription = 'Re-plans the subtree (root kept, children replaced). No auto-run.'
  readonly noPlan = true

  async execute(args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> {
    const context = agentContext.context
    const targetTaskId = String(args?.targetTaskId || '').trim()
    if (!targetTaskId) {
      return { content: [{ type: 'text', text: 'replan_subtree: targetTaskId is required' }], isError: true }
    }

    if (!context.getTaskContext || !context.spawnPlannedTree) {
      return {
        content: [{ type: 'text', text: 'replan_subtree: environment not ready (missing Eko bindings).' }],
        isError: true,
      }
    }

    const targetCtx = context.getTaskContext(targetTaskId)
    if (!targetCtx || !targetCtx.task) {
      return {
        content: [{ type: 'text', text: `replan_subtree: target task not found: ${targetTaskId}` }],
        isError: true,
      }
    }

    const prompt = String(args?.prompt || targetCtx.task.description || targetCtx.task.name || '').trim()
    if (!prompt) {
      return {
        content: [{ type: 'text', text: 'replan_subtree: prompt is empty (and no existing description available).' }],
        isError: true,
      }
    }

    // 1) Plan new tree for target (root)
    const planner = new TreePlanner(targetCtx)
    let planned
    try {
      planned = await planner.planTree(prompt)
    } catch (e) {
      const errText = e instanceof Error ? e.message : String(e)
      return { content: [{ type: 'text', text: `replan_subtree planning failed: ${errText}` }], isError: true }
    }

    // 2) Delete old subtree children (not the root)
    const removedTaskIds: string[] = []
    const collectDescendants = (id: string) => {
      const c = context.getTaskContext!(id)
      const children = c?.task?.childTaskIds || []
      for (const cid of children) {
        collectDescendants(cid)
        // delete leaf after its children collected
        if (context.deleteTask) {
          context.deleteTask(cid)
          removedTaskIds.push(cid)
        }
      }
      // after deleting children, clear child list in memory for UI coherence
      if (c?.task) {
        c.task.childTaskIds = []
      }
    }
    collectDescendants(targetTaskId)

    // 3) Update root (target) with new agent xml/nodes
    try {
      const agentXml = buildAgentXmlFromPlanned(planned)
      targetCtx.task.xml = agentXml
      if (planned.name) targetCtx.task.name = planned.name.slice(0, 80)
      if (planned.description) targetCtx.task.description = planned.description
      const parsed = parseTask(targetTaskId, agentXml, false)
      if (parsed && context.config.callback) {
        parsed.rootTaskId = targetCtx.task.rootTaskId
        parsed.parentTaskId = targetCtx.task.parentTaskId
        parsed.childTaskIds = []
        await context.config.callback.onMessage({
          type: 'task',
          taskId: targetTaskId,
          agentName: agentContext.agent.Name,
          streamDone: true,
          task: parsed,
        })
      }
    } catch (_) {
      // best-effort
    }

    // 4) Spawn subtree per new plan (silent batch to reduce UI event noise)
    const { allTaskIds, leafTaskIds } = await context.spawnPlannedTree!(targetTaskId, planned, { silent: true })

    // 5) Notify UI about tree update
    if (context.config.callback) {
      await context.config.callback.onMessage({
        type: 'task_tree_update',
        taskId: targetTaskId,
        agentName: agentContext.agent.Name,
        parentTaskId: targetTaskId,
        childTaskIds: context.getTaskContext!(targetTaskId)?.task?.childTaskIds || [],
        removedTaskIds,
      })
    }

    // 6) Execution is controlled by caller tools (e.g., new_task). Replan does not auto-run.

    return {
      content: [
        {
          type: 'text',
          text: `Replanned subtree at ${targetTaskId}. Removed: ${removedTaskIds.length}, New tasks: ${allTaskIds.length}, Leaves: ${leafTaskIds.length}.`,
        },
      ],
    }
  }
}
