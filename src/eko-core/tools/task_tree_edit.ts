import { JSONSchema7 } from '../types'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import { buildAgentXmlFromPlanned, parseTask } from '../common/xml'

export const TOOL_NAME = 'task_tree_edit'

type EditOperation = 'add_child' | 'delete_subtree' | 'move_subtree' | 'update_task'

export default class TaskTreeEditTool implements Tool {
  readonly name = TOOL_NAME
  readonly description = 'Edit the task tree: add/delete/move subtree or update a task.'
  readonly parameters: JSONSchema7 = {
    type: 'object',
    properties: {
      operation: {
        type: 'string',
        enum: ['add_child', 'delete_subtree', 'move_subtree', 'update_task'],
        description: 'Task tree edit operation',
      },
      parentTaskId: { type: 'string', description: 'Required for add_child.' },
      targetTaskId: { type: 'string', description: 'Target task for delete_subtree/move_subtree/update_task.' },
      newParentTaskId: { type: 'string', description: 'New parent for move_subtree.' },
      name: { type: 'string', description: 'New name (update_task or add_child).' },
      description: { type: 'string', description: 'New description (update_task or add_child).' },
      nodes: {
        type: 'array',
        items: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
        description: 'New nodes list (update_task or add_child).',
      },
    },
    required: ['operation'],
  }
  readonly planDescription = 'Modifies the task tree structure or task content.'
  readonly noPlan = true

  async execute(args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> {
    const context = agentContext.context
    const op = String(args?.operation || '').trim() as EditOperation
    if (!op) return { content: [{ type: 'text', text: 'task_tree_edit: operation is required' }], isError: true }

    if (!context.getTaskContext || !context.deleteTask || !context.spawnChildTask) {
      return { content: [{ type: 'text', text: 'task_tree_edit: environment not ready (missing Eko bindings).' }], isError: true }
    }

    switch (op) {
      case 'add_child': {
        const parentTaskId = String(args?.parentTaskId || '').trim()
        if (!parentTaskId) return { content: [{ type: 'text', text: 'add_child: parentTaskId is required' }], isError: true }
        const name = (args?.name as string | undefined)?.trim()
        const description = (args?.description as string | undefined)?.trim()
        const nodes = (Array.isArray(args?.nodes) ? (args?.nodes as Array<{ text: string }>) : undefined)?.filter(n => n && typeof n.text === 'string' && n.text.trim().length > 0)

        // spawn child task
        const childId = await context.spawnChildTask(parentTaskId, description || name || 'Subtask', { pauseParent: false })
        const childCtx = context.getTaskContext(childId)
        if (childCtx?.task) {
          if (name) childCtx.task.name = name.slice(0, 80)
          if (description) childCtx.task.description = description
          if (nodes && nodes.length > 0) {
            try {
              const agentXml = buildAgentXmlFromPlanned({ name, description, nodes })
              childCtx.task.xml = agentXml
              const parsed = parseTask(childId, agentXml, false)
              if (parsed && context.config.callback) {
                parsed.rootTaskId = childCtx.task.rootTaskId
                parsed.parentTaskId = childCtx.task.parentTaskId
                parsed.childTaskIds = childCtx.task.childTaskIds
                await context.config.callback.onMessage({
                  type: 'task',
                  taskId: childId,
                  agentName: agentContext.agent.Name,
                  streamDone: true,
                  task: parsed,
                })
              }
            } catch {
              // ignore
            }
          }
        }
        return { content: [{ type: 'text', text: `Added child task ${childId} to ${parentTaskId}` }] }
      }

      case 'delete_subtree': {
        const targetTaskId = String(args?.targetTaskId || '').trim()
        if (!targetTaskId) return { content: [{ type: 'text', text: 'delete_subtree: targetTaskId is required' }], isError: true }
        const targetCtx = context.getTaskContext(targetTaskId)
        if (!targetCtx?.task) return { content: [{ type: 'text', text: `delete_subtree: task not found: ${targetTaskId}` }], isError: true }
        const oldParentId = targetCtx.task.parentTaskId

        const removed: string[] = []
        const dfsDelete = (id: string) => {
          const c = context.getTaskContext!(id)
          const children = c?.task?.childTaskIds || []
          for (const cid of children) dfsDelete(cid)
          if (context.deleteTask!(id)) removed.push(id)
        }
        // delete all descendants first
        const children = targetCtx.task.childTaskIds || []
        for (const cid of children) dfsDelete(cid)
        // then delete the root itself
        if (context.deleteTask!(targetTaskId)) removed.push(targetTaskId)

        // update old parent child list
        if (oldParentId) {
          const oldParent = context.getTaskContext(oldParentId)
          if (oldParent?.task) {
            oldParent.task.childTaskIds = (oldParent.task.childTaskIds || []).filter(id => id !== targetTaskId)
          }
          if (context.config.callback) {
            await context.config.callback.onMessage({
              type: 'task_tree_update',
              taskId: oldParentId,
              agentName: agentContext.agent.Name,
              parentTaskId: oldParentId,
              childTaskIds: (oldParent?.task?.childTaskIds || []),
              removedTaskIds: removed,
            })
          }
        }

        return { content: [{ type: 'text', text: `Deleted subtree ${targetTaskId} (removed ${removed.length} tasks)` }] }
      }

      case 'move_subtree': {
        const targetTaskId = String(args?.targetTaskId || '').trim()
        const newParentTaskId = String(args?.newParentTaskId || '').trim()
        if (!targetTaskId || !newParentTaskId) {
          return { content: [{ type: 'text', text: 'move_subtree: targetTaskId and newParentTaskId are required' }], isError: true }
        }
        const targetCtx = context.getTaskContext(targetTaskId)
        const newParentCtx = context.getTaskContext(newParentTaskId)
        if (!targetCtx?.task || !newParentCtx?.task) {
          return { content: [{ type: 'text', text: 'move_subtree: target or new parent not found' }], isError: true }
        }
        const oldParentId = targetCtx.task.parentTaskId

        // detach from old parent
        if (oldParentId) {
          const oldParent = context.getTaskContext(oldParentId)
          if (oldParent?.task) {
            oldParent.task.childTaskIds = (oldParent.task.childTaskIds || []).filter(id => id !== targetTaskId)
          }
        }

        // attach to new parent
        targetCtx.attachParent(newParentTaskId, newParentCtx.task.rootTaskId || newParentTaskId)
        newParentCtx.addChild(targetTaskId)

        // update rootTaskId across descendants if root changed
        const newRoot = newParentCtx.task.rootTaskId || newParentTaskId
        const bfs = (id: string) => {
          const c = context.getTaskContext!(id)
          if (c?.task) c.task.rootTaskId = newRoot
          const children = c?.task?.childTaskIds || []
          for (const cid of children) bfs(cid)
        }
        bfs(targetTaskId)

        // notify UI
        if (oldParentId && context.config.callback) {
          const oldParent = context.getTaskContext(oldParentId)
          await context.config.callback.onMessage({
            type: 'task_tree_update',
            taskId: oldParentId,
            agentName: agentContext.agent.Name,
            parentTaskId: oldParentId,
            childTaskIds: (oldParent?.task?.childTaskIds || []),
          })
        }
        if (context.config.callback) {
          await context.config.callback.onMessage({
            type: 'task_tree_update',
            taskId: newParentTaskId,
            agentName: agentContext.agent.Name,
            parentTaskId: newParentTaskId,
            childTaskIds: (newParentCtx.task.childTaskIds || []),
          })
        }

        return { content: [{ type: 'text', text: `Moved subtree ${targetTaskId} under ${newParentTaskId}` }] }
      }

      case 'update_task': {
        const targetTaskId = String(args?.targetTaskId || '').trim()
        if (!targetTaskId) return { content: [{ type: 'text', text: 'update_task: targetTaskId is required' }], isError: true }
        const targetCtx = context.getTaskContext(targetTaskId)
        if (!targetCtx?.task) return { content: [{ type: 'text', text: `update_task: task not found: ${targetTaskId}` }], isError: true }

        const name = (args?.name as string | undefined)?.trim()
        const description = (args?.description as string | undefined)?.trim()
        const nodes = (Array.isArray(args?.nodes) ? (args?.nodes as Array<{ text: string }>) : undefined)?.filter(n => n && typeof n.text === 'string' && n.text.trim().length > 0)

        if (name) targetCtx.task.name = name.slice(0, 80)
        if (description) targetCtx.task.description = description
        if (nodes && nodes.length > 0) {
          try {
            const agentXml = buildAgentXmlFromPlanned({ name, description, nodes })
            targetCtx.task.xml = agentXml
          } catch {
            // ignore xml build errors
          }
        }

        // emit task event for UI refresh
        if (context.config.callback) {
          const parsed = parseTask(targetTaskId, targetCtx.task.xml, false)
          await context.config.callback.onMessage({
            type: 'task',
            taskId: targetTaskId,
            agentName: agentContext.agent.Name,
            streamDone: true,
            task: parsed || targetCtx.task,
          })
        }

        return { content: [{ type: 'text', text: `Updated task ${targetTaskId}` }] }
      }
    }

    return { content: [{ type: 'text', text: `task_tree_edit: unsupported operation ${op}` }], isError: true }
  }
}
