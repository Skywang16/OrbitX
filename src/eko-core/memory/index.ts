import config from '../config'
import { Tool, NativeLLMMessage, NativeLLMTool, NativeLLMToolCall } from '../types'
import TaskSnapshotTool from './snapshot'
import { RetryLanguageModel } from '../llm'
import { mergeTools } from '../common/utils'
import { AgentContext, generateNodeId } from '../core/context'
import Log from '../common/log'

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

export function removeDuplicateToolUse(
  results: Array<
    | { type: 'text'; text: string }
    | { type: 'tool-call'; toolCallId: string; toolName: string; args: Record<string, unknown> }
  >
): Array<
  | { type: 'text'; text: string }
  | { type: 'tool-call'; toolCallId: string; toolName: string; args: Record<string, unknown> }
> {
  if (results.length <= 1 || results.filter(r => r.type == 'tool-call').length <= 1) {
    return results
  }
  let _results = []
  let tool_uniques = []
  for (let i = 0; i < results.length; i++) {
    if (results[i].type === 'tool-call') {
      let tool = results[i] as {
        type: 'tool-call'
        toolCallId: string
        toolName: string
        args: Record<string, unknown>
      }
      let key = tool.toolName + JSON.stringify(tool.args)
      if (tool_uniques.indexOf(key) == -1) {
        _results.push(results[i])
        tool_uniques.push(key)
      }
    } else {
      _results.push(results[i])
    }
  }
  return _results
}

export async function compressAgentMessages(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  callAgentLLM: any
) {
  if (messages.length < 5) {
    return
  }
  try {
    await doCompressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
  } catch (e) {
    Log.error('Error compressing agent messages:', e)
  }
}

async function doCompressAgentMessages(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  callAgentLLM: any
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
  const result = await callAgentLLM(agentContext, rlm, newMessages, newTools, true, {
    type: 'tool',
    toolName: snapshotTool.name,
  })
  const toolCall = result.filter((s: any) => s.type == 'tool-call')[0]
  const args = typeof toolCall.args == 'string' ? JSON.parse(toolCall.args || '{}') : toolCall.args || {}
  const nativeToolCall: NativeLLMToolCall = {
    id: toolCall.toolCallId,
    name: toolCall.toolName,
    arguments: args,
  }
  const toolResult = await snapshotTool.execute(args, agentContext, nativeToolCall)
  const callback = agentContext.context.config.callback
  if (callback) {
    const toolResultNodeId =
      agentContext.context.currentNodeId || generateNodeId(agentContext.context.taskId, 'execution')
    await callback.onMessage(
      {
        taskId: agentContext.context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: toolResultNodeId,
        type: 'tool_result',
        toolId: toolCall.toolCallId,
        toolName: toolCall.toolName,
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
  // system, user, assistant, tool(first), [...], <user>, assistant, tool(last), ...
  const textContent = toolResult.content
    .filter(s => s.type == 'text')
    .map(s => ({ type: 'text' as const, text: (s as any).text }))
  messages.splice(firstToolIndex + 1, lastToolIndex - firstToolIndex - 2, {
    role: 'user',
    content: textContent,
  })
}

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
