import { JSONSchema7, NativeLLMToolCall, NativeLLMMessage, NativeLLMTool } from '../types'
import { RetryLanguageModel } from '../llm'
import { extractUsedTool } from '../memory'
import { mergeTools } from '../common/utils'
import { callAgentLLM } from '../agent/llm'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import Log from '../common/log'

export const TOOL_NAME = 'task_result_check'

export default class TaskResultCheckTool implements Tool {
  readonly name: string = TOOL_NAME
  readonly description: string
  readonly parameters: JSONSchema7

  constructor() {
    this.description = `Check the current task execution process and results, evaluate the overall completion status of the current task, and whether the output variables in the nodes are stored.`
    this.parameters = {
      type: 'object',
      properties: {
        thought: {
          type: 'string',
          description:
            'Please conduct thoughtful analysis of the overall execution process and results of the current task, analyzing whether the task has been completed.',
        },
        completionStatus: {
          type: 'string',
          description:
            'The completion status of the current task is only considered complete when the entire current task is finished; partial completion or task failure is considered incomplete',
          enum: ['completed', 'incomplete'],
        },
        todoList: {
          type: 'string',
          description:
            'Pending task list for incomplete tasks, when tasks are not fully completed, please describe which tasks remain to be completed',
        },
      },
      required: ['thought', 'completionStatus'],
    }
  }

  async execute(
    _args: Record<string, unknown>,
    _agentContext: AgentContext,
    _toolCall?: NativeLLMToolCall
  ): Promise<ToolResult> {
    return {
      content: [
        {
          type: 'text',
          text: 'success',
        },
      ],
    }
  }
}

async function doTaskResultCheck(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[]
): Promise<{ completionStatus: 'completed' | 'incomplete' }> {
  try {
    // extract used tool
    const usedTools = extractUsedTool(messages, tools)
    const taskResultCheck = new TaskResultCheckTool()
    const newTools = mergeTools(usedTools, [
      {
        name: taskResultCheck.name,
        description: taskResultCheck.description,
        parameters: taskResultCheck.parameters,
      },
    ])
    // handle messages
    const newMessages: NativeLLMMessage[] = [...messages]
    newMessages.push({
      role: 'user',
      content: [
        {
          type: 'text',
          text: `Task:\n${agentContext.context.task?.xml || ''}\n\nPlease check the completion status of the current task.`,
        },
      ],
    })
    const result = await callAgentLLM(agentContext, rlm, newMessages, newTools, true, taskResultCheck.name)
    const toolCall = result.toolCalls[0]
    if (!toolCall) {
      throw new Error('No tool call found in result')
    }
    const args = toolCall.arguments || {}
    const nativeToolCall: NativeLLMToolCall = toolCall
    const toolResult = await taskResultCheck.execute(args, agentContext, nativeToolCall)
    const callback = agentContext.context.config.callback
    if (callback) {
      await callback.onMessage(
        {
          taskId: agentContext.context.taskId,
          agentName: agentContext.agent.Name,
          nodeId: agentContext.context.taskId,
          type: 'tool_result',
          toolId: toolCall.id,
          toolName: toolCall.name,
          params: args,
          toolResult: toolResult,
        },
        agentContext
      )
    }
    if (args.completionStatus == 'incomplete') {
      messages.push({
        role: 'user',
        content: [
          {
            type: 'text',
            text: `It seems that your task has not been fully completed. Please continue with the remaining steps:\n${
              args.todoList || ''
            }`,
          },
        ],
      })
    }
    return {
      completionStatus: args.completionStatus as 'completed' | 'incomplete',
    }
  } catch (e) {
    Log.error('TaskResultCheckTool error', e instanceof Error ? e : String(e))
    return {
      completionStatus: 'completed',
    }
  }
}

export { TaskResultCheckTool, doTaskResultCheck }
