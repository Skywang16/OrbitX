import { JSONSchema7, NativeLLMToolCall, NativeLLMMessage, NativeLLMTool } from '../types'
import { RetryLanguageModel } from '../llm'
import { extractUsedTool } from '../memory'
import { mergeTools } from '../common/utils'
import { callAgentLLM } from '../agent/llm'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import Log from '../common/log'

export const TOOL_NAME = 'todo_list_manager'

export default class TodoListManagerTool implements Tool {
  readonly name: string = TOOL_NAME
  readonly description: string
  readonly parameters: JSONSchema7

  constructor() {
    this.description =
      'Current task to-do list management, used for managing the to-do list of current tasks. During task execution, the to-do list needs to be updated according to the task execution status: completed, pending. It also detects whether tasks are being executed in repetitive loops during the execution process.'
    this.parameters = {
      type: 'object',
      properties: {
        completedList: {
          type: 'array',
          description:
            'Current completed task list items. Please update the completed list items based on the current task completion status.',
          items: {
            type: 'string',
          },
        },
        todoList: {
          type: 'array',
          description:
            'Current pending task list items. Please update the pending list items based on the current task pending status.',
          items: {
            type: 'string',
          },
        },
        loopDetection: {
          type: 'string',
          description: 'Check if the current step is being repeatedly executed by comparing with previous steps.',
          enum: ['loop', 'no_loop'],
        },
      },
      required: ['completedList', 'todoList', 'loopDetection'],
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

async function doTodoListManager(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[]
) {
  try {
    // extract used tool
    const usedTools = extractUsedTool(messages, tools)
    const todoListManager = new TodoListManagerTool()
    const newTools = mergeTools(usedTools, [
      {
        name: todoListManager.name,
        description: todoListManager.description,
        parameters: todoListManager.parameters,
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
    const result = await callAgentLLM(agentContext, rlm, newMessages, newTools, true, todoListManager.name)
    const toolCall = result.find(s => 'id' in s && 'name' in s) as NativeLLMToolCall
    if (!toolCall) {
      throw new Error('No tool call found in result')
    }
    const args = toolCall.arguments || {}
    const nativeToolCall: NativeLLMToolCall = toolCall
    const toolResult = await todoListManager.execute(args, agentContext, nativeToolCall)
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
    let userPrompt = '# Task Execution Status\n'
    const completedList = args.completedList as string[]
    const todoList = args.todoList as string[]
    if (completedList && completedList.length > 0) {
      userPrompt += '## Completed task list\n'
      for (let i = 0; i < completedList.length; i++) {
        userPrompt += `- ${completedList[i]}\n`
      }
      userPrompt += '\n'
    }
    if (todoList && todoList.length > 0) {
      userPrompt += '## Pending task list\n'
      for (let i = 0; i < todoList.length; i++) {
        userPrompt += `- ${todoList[i]}\n`
      }
      userPrompt += '\n'
    }
    if (args.loopDetection == 'loop') {
      userPrompt += `## Loop detection\nIt seems that your task is being executed in a loop, Please change the execution strategy and try other methods to complete the current task.\n\n`
    }
    userPrompt += 'Please continue executing the remaining tasks.'
    messages.push({
      role: 'user',
      content: [
        {
          type: 'text',
          text: userPrompt.trim(),
        },
      ],
    })
  } catch (e) {
    Log.error('TodoListManagerTool error', e instanceof Error ? e : String(e))
  }
}

export { TodoListManagerTool, doTodoListManager }
