import Log from '../common/log'
import {
  EkoMessage,
  ToolResult,
  DialogueTool,
  DialogueParams,
  DialogueCallback,
  EkoDialogueConfig,
  EkoMessageUserPart,
} from '../types'
import { NativeLLMMessagePart, NativeLLMToolCall } from '../types/llm.types'
import { callChatLLM, convertToolResults, convertAssistantToolResults } from './dialogue/llm'
import { Eko } from './eko'
import TaskPlannerTool, { TOOL_NAME as task_planner } from './dialogue/task_planner'
import { RetryLanguageModel } from '../llm'
import { EkoMemory } from '../memory/memory'
import ExecuteTaskTool from './dialogue/execute_task'
import { getDialogueSystemPrompt } from '../prompt'

import { convertTools, getTool, convertToolResult } from '../llm/conversion-utils'

export class EkoDialogue {
  protected memory: EkoMemory
  protected tools: DialogueTool[]
  protected config: EkoDialogueConfig
  protected ekoMap: Map<string, Eko>
  protected globalContext: Map<string, any>

  constructor(config: EkoDialogueConfig, memory?: EkoMemory, tools?: DialogueTool[]) {
    this.config = config
    this.tools = tools ?? []
    this.ekoMap = new Map<string, Eko>()
    this.globalContext = new Map<string, any>()
    this.memory = memory ?? new EkoMemory(getDialogueSystemPrompt())
  }

  public async chat(params: DialogueParams): Promise<string> {
    return this.doChat(params, false)
  }

  public async segmentedExecution(params: Omit<DialogueParams, 'user'>): Promise<string> {
    const messages = this.memory.getMessages()
    const lastMessage = messages[messages.length - 1]
    if (lastMessage.role !== 'tool' || !lastMessage.content.some(part => part.toolName === task_planner)) {
      throw new Error('No task planner tool call found')
    }
    const userMessages = messages.filter(message => message.role === 'user')
    const lastUserMessage = userMessages[userMessages.length - 1]
    if (!lastUserMessage) {
      throw new Error('No user message found')
    }
    return this.doChat(
      {
        ...params,
        user: lastUserMessage.content as string | EkoMessageUserPart[],
        callback: params.callback,
        messageId: params.messageId || lastUserMessage.id,
        signal: params.signal,
      },
      true
    )
  }

  private async doChat(params: DialogueParams, segmentedExecution: boolean): Promise<string> {
    if (!segmentedExecution) {
      params.messageId = params.messageId ?? this.memory.genMessageId()
      await this.addUserMessage(params.user, params.messageId)
    }
    const rlm = new RetryLanguageModel(this.config.llms, this.config.chatLlms)
    for (let i = 0; i < 15; i++) {
      const messages = this.memory.buildMessages()
      const chatTools = [...this.buildInnerTools(params), ...this.tools]
      const results = await callChatLLM(
        params.messageId as string,
        rlm,
        messages,
        convertTools(chatTools),
        undefined,
        0,
        params.callback,
        params.signal
      )
      const finalResult = await this.handleCallResult(chatTools, results, params.callback)
      if (finalResult) {
        return finalResult
      }
      if (this.config.segmentedExecution && results.some(r => r.type == 'tool-call' && r.toolName == task_planner)) {
        return 'segmentedExecution'
      }
    }
    return 'Unfinished'
  }

  protected async addUserMessage(user: string | EkoMessageUserPart[], messageId: string): Promise<EkoMessage> {
    const message: EkoMessage = {
      id: messageId,
      role: 'user',
      timestamp: Date.now(),
      content: user,
    }
    await this.memory.addMessages([message])
    return message
  }

  protected buildInnerTools(params: DialogueParams): DialogueTool[] {
    return [new TaskPlannerTool(this, params), new ExecuteTaskTool(this)]
  }

  public addEko(taskId: string, eko: Eko): void {
    this.ekoMap.set(taskId, eko)
  }

  public getEko(taskId: string): Eko | undefined {
    return this.ekoMap.get(taskId)
  }

  public getGlobalContext(): Map<string, unknown> {
    return this.globalContext
  }

  public getConfig(): EkoDialogueConfig {
    return this.config
  }

  protected async handleCallResult(
    chatTools: DialogueTool[],
    results: NativeLLMMessagePart[],
    dialogueCallback?: DialogueCallback
  ): Promise<string | null> {
    let text: string | null = null
    const toolResults: NativeLLMMessagePart[] = []
    if (results.length == 0) {
      return null
    }
    for (let i = 0; i < results.length; i++) {
      const result = results[i]
      if (result.type == 'text') {
        text = result.text || null
        continue
      }
      if (result.type == 'tool-call') {
        let toolResult: ToolResult
        try {
          const args = result.args || {}
          const tool = getTool(chatTools, result.toolName || '')
          if (!tool) {
            throw new Error((result.toolName || 'unknown') + ' tool does not exist')
          }

          // Create NativeLLMToolCall for compatibility
          const toolCall: NativeLLMToolCall = {
            id: result.toolCallId || '',
            name: result.toolName || '',
            arguments: args,
          }

          toolResult = await tool.execute(args, toolCall)
        } catch (e) {
          Log.error('tool call error: ', result.toolName, result.args, e)
          toolResult = {
            content: [
              {
                type: 'text',
                text: e + '',
              },
            ],
            isError: true,
          }
        }
        const callback = dialogueCallback?.chatCallback
        if (callback) {
          await callback.onMessage({
            type: 'tool_result',
            toolId: result.toolCallId || '',
            toolName: result.toolName || '',
            params: (result.args as Record<string, unknown>) || {},
            toolResult: toolResult,
          })
        }

        // Create tool call for conversion
        const toolCall: NativeLLMToolCall = {
          id: result.toolCallId || '',
          name: result.toolName || '',
          arguments: result.args || {},
        }

        const llmToolResult = convertToolResult(toolCall, toolResult)
        toolResults.push(llmToolResult)
      }
    }
    await this.memory.addMessages([
      {
        id: this.memory.genMessageId(),
        role: 'assistant',
        timestamp: Date.now(),
        content: convertAssistantToolResults(results),
      },
    ])
    if (toolResults.length > 0) {
      await this.memory.addMessages([
        {
          id: this.memory.genMessageId(),
          role: 'tool',
          timestamp: Date.now(),
          content: convertToolResults(toolResults),
        },
      ])
      return null
    } else {
      return text
    }
  }
}
