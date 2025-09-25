import { JSONSchema7 } from '../types'
import type { PlannedTask } from '../types'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import { TreePlanner } from '../core/plan-tree'

export const TOOL_NAME = 'new_task'

/**
 * Plan a TWO-LEVEL task tree under the current task and execute all leaf subtasks sequentially.
 * Flow: plan -> spawn silently (parent -> groups -> subtasks) -> pause parent -> run leaves in order -> resume parent
 */
export default class NewTaskTool implements Tool {
  readonly name = TOOL_NAME
  readonly description =
    'Plan a two-level task tree (groups -> subtasks), create tasks under the current task, and execute all subtasks sequentially.'
  readonly parameters: JSONSchema7 = {
    type: 'object',
    properties: {
      message: {
        type: 'string',
        description: 'High-level objective; the planner will produce groups and subtasks with concrete steps.',
      },
    },
    required: ['message'],
  }
  readonly planDescription = 'Creates a two-level task plan and runs all leaf subtasks sequentially.'
  readonly noPlan = true

  async execute(args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> {
    const context = agentContext.context
    const message = String(args?.message || '').trim()
    if (!message) {
      return {
        content: [{ type: 'text', text: 'new_task: parameter "message" is required' }],
        isError: true,
      }
    }

    if (!context.spawnPlannedTree || !context.executeTask) {
      return {
        content: [{ type: 'text', text: 'new_task: environment not ready (missing Eko bindings).' }],
        isError: true,
      }
    }

    const planner = new TreePlanner(context)
    let planned: PlannedTask
    try {
      planned = await planner.planTree(message)
    } catch (e) {
      const errText = e instanceof Error ? e.message : String(e)
      return { content: [{ type: 'text', text: `Tree planning failed: ${errText}` }], isError: true }
    }

    // Sanitize plan: ensure names/descriptions, clamp to two levels
    const sanitize = (plan: PlannedTask, depth: number): PlannedTask => {
      const p: PlannedTask = { ...plan }
      if (!p.name || !String(p.name).trim()) p.name = (p.description && String(p.description).trim()) || ''
      if (!p.description || !String(p.description).trim()) p.description = p.name || ''
      if (depth >= 2) {
        // drop deeper nesting
        delete p.subtasks
        return p
      }
      if (Array.isArray(p.subtasks)) {
        p.subtasks = p.subtasks.map((child: PlannedTask) => sanitize(child, depth + 1))
      }
      return p
    }
    planned = sanitize({ ...planned, name: planned.name || message, description: planned.description || message }, 0)

    const parentTaskId = context.taskId
    const { allTaskIds, leafTaskIds } = await context.spawnPlannedTree(parentTaskId, planned, { silent: true })

    // Pause parent and execute leaves sequentially in the background to avoid blocking tool_result
    const cb = context.config.callback
    ;(async () => {
      try {
        // Pause parent (UI hint)
        context.setPause(true)
        await cb?.onMessage({
          type: 'task_pause',
          taskId: parentTaskId,
          agentName: agentContext.agent.Name,
          nodeId: context.currentNodeId,
          reason: 'new_task_seq_start',
        })

        for (const id of leafTaskIds) {
          try {
            const res = await context.executeTask!(id)
            // Stream child result to parent
            await cb?.onMessage({
              type: 'task_child_result',
              taskId: id,
              agentName: agentContext.agent.Name,
              nodeId: context.currentNodeId,
              parentTaskId: parentTaskId,
              summary: res.result,
            })
          } catch (e) {
            await cb?.onMessage({
              type: 'error',
              taskId: parentTaskId,
              agentName: agentContext.agent.Name,
              nodeId: context.currentNodeId,
              error: e instanceof Error ? e.message : String(e),
            })
          }
        }
      } finally {
        context.setPause(false)
        await cb?.onMessage({
          type: 'task_resume',
          taskId: parentTaskId,
          agentName: agentContext.agent.Name,
          nodeId: context.currentNodeId,
          reason: 'new_task_seq_done',
        })
      }
    })()

    return {
      content: [
        {
          type: 'text',
          text: `Created ${allTaskIds.length} tasks (${leafTaskIds.length} subtasks). Executing subtasks sequentially...`,
        },
      ],
    }
  }
}
