import { JSONSchema7 } from '../types'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'

export const TOOL_NAME = 'new_task'

/**
 * Spawn a subtask with isolated context and objective.
 * MVP: serial execution (pause parent -> run child -> resume parent)
 */
export default class NewTaskTool implements Tool {
  readonly name = TOOL_NAME
  readonly description =
    'Spawn a subtask with an isolated context. Use when you need exploration, long-running work, or to isolate effects.'
  readonly parameters: JSONSchema7 = {
    type: 'object',
    properties: {
      message: {
        type: 'string',
        description: 'Subtask objective/instruction',
      },
      mode: {
        type: 'string',
        enum: ['code', 'debug', 'architect', 'general'],
        default: 'general',
        description: 'Optional execution mode for the child task (reserved for future use).',
      },
      todos: {
        type: 'array',
        items: { type: 'string' },
        description: 'Optional initial todos to seed the child task with.',
      },
    },
    required: ['message'],
  }
  readonly planDescription =
    'Creates a child task, pauses the parent, runs the child to completion, then resumes the parent with a summary.'
  readonly noPlan = true

  async execute(args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> {
    const context = agentContext.context
    const message = String(args?.message || '').trim()
    if (!message) {
      return {
        content: [
          {
            type: 'text',
            text: 'new_task: parameter "message" is required',
          },
        ],
        isError: true,
      }
    }

    if (!context.spawnChildTask || !context.completeChildTask || !context.executeTask) {
      return {
        content: [
          {
            type: 'text',
            text: 'new_task: environment not ready (missing Eko bindings).',
          },
        ],
        isError: true,
      }
    }

    const parentTaskId = context.taskId
    // 1) spawn child
    const childTaskId = await context.spawnChildTask(parentTaskId, message)

    // 2) run child to completion
    let childResultText = ''
    try {
      const execResult = await context.executeTask(childTaskId)
      childResultText = execResult.result || ''
    } catch (e) {
      // Even on error, attempt to resume parent with an error summary
      const errText = e instanceof Error ? e.message : String(e)
      await context.completeChildTask(childTaskId, `[Error] ${errText}`)
      return {
        content: [
          {
            type: 'text',
            text: `Child task ${childTaskId} failed: ${errText}`,
          },
        ],
        isError: true,
      }
    }

    // 3) flow summary back to parent and resume
    const summary = childResultText?.slice(0, 1000) || 'Child task completed.'
    await context.completeChildTask(childTaskId, summary)

    return {
      content: [
        {
          type: 'text',
          text: `Spawned and executed subtask: ${childTaskId}`,
        },
      ],
    }
  }
}
