import config from '../config'
import { Tool, NativeLLMMessage, NativeLLMTool, NativeLLMToolCall } from '../types'
import TaskSnapshotTool from './snapshot'
import { RetryLanguageModel } from '../llm'
import { mergeTools } from '../common/utils'
import { AgentContext, generateNodeId } from '../core/context'
import Log from '../common/log'
import type { StreamCallback, HumanCallback, LLMRequest } from '../types'
import type { AgentLLMCallResult } from '../agent/llm'

export function extractUsedTool<T extends Tool | NativeLLMTool>(messages: NativeLLMMessage[], agentTools: T[]): T[] {
  let tools: T[] = []
  let toolNames: string[] = []
  for (let i = 0; i < messages.length; i++) {
    let message = messages[i]
    if (message.role == 'tool') {
      const content = Array.isArray(message.content) ? message.content : []
      for (let j = 0; j < content.length; j++) {
        const part = content[j]
        if (part.type === 'tool-result' && part.toolName) {
          const toolName = part.toolName
          if (toolNames.indexOf(toolName) > -1) {
            continue
          }
          toolNames.push(toolName)
          let tool = agentTools.filter(tool => tool.name === toolName)[0]
          if (tool) {
            tools.push(tool)
          }
        }
      }
    }
  }
  return tools
}

export async function compressAgentMessages(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  callAgentLLM: CallAgentLLM
) {
  if (messages.length < 5) {
    return
  }
  try {
    await doCompressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
  } catch (e) {
    Log.error('Error compressing agent messages:', e instanceof Error ? e : String(e))
  }
}

async function doCompressAgentMessages(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  callAgentLLM: CallAgentLLM
) {
  // extract used tool
  const usedTools = extractUsedTool(messages, tools)
  const snapshotTool = new TaskSnapshotTool()
  const newTools = mergeTools(usedTools, [
    {
      name: snapshotTool.name,
      description: snapshotTool.description,
      parameters: snapshotTool.parameters,
    },
  ])
  // handle messages
  let lastToolIndex = messages.length - 1
  let newMessages: NativeLLMMessage[] = messages
  for (let r = newMessages.length - 1; r > 3; r--) {
    if (newMessages[r].role == 'tool') {
      newMessages = newMessages.slice(0, r + 1)
      lastToolIndex = r
      break
    }
  }
  newMessages.push({
    role: 'user',
    content: [
      {
        type: 'text',
        text: 'Please create a snapshot backup of the current task, keeping only key important information and node completion status.',
      },
    ],
  })
  // compress snapshot
  const result = await callAgentLLM(agentContext, rlm, newMessages, newTools, true, snapshotTool.name)
  const toolCall = result.toolCalls[0]
  if (!toolCall) {
    throw new Error('Snapshot tool was not invoked by the agent')
  }
  const args = toolCall.arguments || {}
  const nativeToolCall: NativeLLMToolCall = {
    id: toolCall.id,
    name: toolCall.name,
    arguments: args,
  }
  const toolResult = await snapshotTool.execute(args, agentContext, nativeToolCall)
  const callback = agentContext.context.config.callback
  if (callback && toolCall) {
    const toolResultNodeId =
      agentContext.context.currentNodeId || generateNodeId(agentContext.context.taskId, 'execution')
    await callback.onMessage(
      {
        taskId: agentContext.context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: toolResultNodeId,
        type: 'tool_result',
        toolId: toolCall.id,
        toolName: toolCall.name,
        params: args,
        toolResult: toolResult,
      },
      agentContext
    )
  }
  // handle original messages
  let firstToolIndex = 3
  for (let i = 0; i < messages.length; i++) {
    if (messages[0].role == 'tool') {
      firstToolIndex = i
      break
    }
  }

  const textItems = toolResult.content.filter(s => s.type === 'text') as Array<{ type: 'text'; text: string }>
  const textContent = textItems.map(s => ({ type: 'text' as const, text: s.text }))
  messages.splice(firstToolIndex + 1, lastToolIndex - firstToolIndex - 2, {
    role: 'user',
    content: textContent,
  })
}

type CallAgentLLM = (
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  noCompress?: boolean,
  toolChoice?: string,
  retryNum?: number,
  callback?: StreamCallback & HumanCallback,
  requestHandler?: (request: LLMRequest) => void
) => Promise<AgentLLMCallResult>

export function handleLargeContextMessages(messages: NativeLLMMessage[]) {
  let imageNum = 0
  let fileNum = 0
  let maxNum = config.maxDialogueImgFileNum
  let longTextTools: Record<string, number> = {}
  for (let i = messages.length - 1; i >= 0; i--) {
    let message = messages[i]
    if (message.role == 'user') {
      const content = Array.isArray(message.content) ? message.content : []
      for (let j = 0; j < content.length; j++) {
        let part = content[j]
        if (part.type == 'file' && part.mimeType?.startsWith('image/')) {
          if (++imageNum <= maxNum) {
            break
          }
          part = {
            type: 'text',
            text: '[image]',
          }
          content[j] = part
        } else if (part.type == 'file') {
          if (++fileNum <= maxNum) {
            break
          }
          part = {
            type: 'text',
            text: '[file]',
          }
          content[j] = part
        }
      }
    } else if (message.role == 'tool') {
      const content = Array.isArray(message.content) ? message.content : []
      for (let j = 0; j < content.length; j++) {
        let toolResult = content[j]
        if (toolResult.type === 'tool-result' && toolResult.toolName) {
          if (typeof toolResult.result === 'string' && toolResult.result.length > config.largeTextLength) {
            if (!longTextTools[toolResult.toolName]) {
              longTextTools[toolResult.toolName] = 1
            } else {
              longTextTools[toolResult.toolName]++
            }
            if (longTextTools[toolResult.toolName] > 1) {
              toolResult.result = toolResult.result.substring(0, config.largeTextLength) + '...'
            }
          }
        }
      }
    }
  }
}
