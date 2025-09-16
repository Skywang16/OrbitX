import config from '../config'
import Log from '../common/log'
import * as memory from '../memory'
import { RetryLanguageModel } from '../llm'
import { AgentContext, generateNodeId } from '../core/context'
import { uuidv4, sleep } from '../common/utils'
import {
  LLMRequest,
  StreamCallback,
  HumanCallback,
  StreamResult,
  NativeLLMMessage,
  NativeLLMTool,
  NativeLLMToolCall,
  NativeLLMStreamChunk,
  FinishReason,
} from '../types'
import { convertTools, getTool, convertToolResult } from '../llm/conversion-utils'

// Export conversion utilities from llm module
export { convertTools, getTool, convertToolResult }

export function removeDuplicateToolUse(
  results: Array<{ type: 'text'; text: string } | NativeLLMToolCall>
): Array<{ type: 'text'; text: string } | NativeLLMToolCall> {
  if (results.length <= 1) {
    return results
  }

  const toolCalls = results.filter(r => 'id' in r) as NativeLLMToolCall[]
  if (toolCalls.length <= 1) {
    return results
  }

  const _results: Array<{ type: 'text'; text: string } | NativeLLMToolCall> = []
  const tool_uniques: string[] = []

  for (const result of results) {
    if ('id' in result) {
      const key = result.name + JSON.stringify(result.arguments)
      if (tool_uniques.indexOf(key) === -1) {
        _results.push(result)
        tool_uniques.push(key)
      }
    } else {
      _results.push(result)
    }
  }

  return _results
}

export async function callAgentLLM(
  agentContext: AgentContext,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  noCompress?: boolean,
  toolChoice?: string,
  retryNum: number = 0,
  callback?: StreamCallback & HumanCallback,
  requestHandler?: (request: LLMRequest) => void
): Promise<Array<{ type: 'text'; text: string } | NativeLLMToolCall>> {
  await agentContext.context.checkAborted()
  if (messages.length >= config.compressThreshold && !noCompress) {
    await memory.compressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
  }
  if (!toolChoice) {
    // Append user dialogue
    appendUserConversation(agentContext, messages)
  }
  let context = agentContext.context

  let streamCallback = callback ||
    context.config.callback || {
      onMessage: async () => {},
    }

  const stepController = new AbortController()
  const signal = AbortSignal.any([context.controller.signal, stepController.signal])
  let request: LLMRequest = {
    tools: tools,
    toolChoice,
    messages: messages,
    abortSignal: signal,
  }
  requestHandler && requestHandler(request)
  // Store request in chain
  agentContext.context.chain.planRequest = request
  let result: StreamResult
  try {
    context.currentStepControllers.add(stepController)
    result = await rlm.callStream(request)
  } catch (e: unknown) {
    context.currentStepControllers.delete(stepController)
    await context.checkAborted()

    // Handle context length errors with compression
    if (!noCompress && messages.length >= 5 && ((e + '').indexOf('tokens') > -1 || (e + '').indexOf('too long') > -1)) {
      await memory.compressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
    }

    // Use enhanced error handling for retry decisions
    const shouldRetry = retryNum < config.maxRetryNum

    if (shouldRetry) {
      // Use exponential backoff with jitter
      const baseDelay = 200
      const delay = Math.min(baseDelay * Math.pow(2, retryNum), 5000) + Math.random() * 1000
      await sleep(delay)
      return callAgentLLM(agentContext, rlm, messages, tools, noCompress, toolChoice, ++retryNum, streamCallback)
    }
    throw e
  }

  let streamText = ''
  let textStreamId = uuidv4()
  let textStreamDone = false
  const toolCalls: NativeLLMToolCall[] = []
  const reader = result.stream.getReader()

  try {
    while (true) {
      await context.checkAborted()
      const { done, value } = await reader.read()
      if (done) {
        break
      }

      const chunk = value as NativeLLMStreamChunk
      switch (chunk.type) {
        case 'delta': {
          // Handle text content
          if (chunk.content) {
            streamText += chunk.content
            await streamCallback.onMessage(
              {
                taskId: context.taskId,
                agentName: agentContext.agent.Name,
                nodeId: agentContext.context.taskId,
                type: 'text',
                streamId: textStreamId,
                streamDone: false,
                text: streamText,
              },
              agentContext
            )
          }

          // Handle tool calls
          if (chunk.toolCalls) {
            for (const toolCall of chunk.toolCalls) {
              // Complete tool call received
              toolCalls.push(toolCall)
              await streamCallback.onMessage(
                {
                  taskId: context.taskId,
                  agentName: agentContext.agent.Name,
                  nodeId: agentContext.context.taskId,
                  type: 'tool_use',
                  toolId: toolCall.id,
                  toolName: toolCall.name,
                  params: toolCall.arguments,
                },
                agentContext
              )
            }
          }
          break
        }
        case 'finish': {
          if (!textStreamDone && streamText) {
            textStreamDone = true
            const textNodeId = agentContext.context.currentNodeId || generateNodeId(context.taskId, 'execution')
            await streamCallback.onMessage(
              {
                taskId: context.taskId,
                agentName: agentContext.agent.Name,
                nodeId: textNodeId,
                type: 'text',
                streamId: textStreamId,
                streamDone: true,
                text: streamText,
              },
              agentContext
            )
          }

          await streamCallback.onMessage(
            {
              taskId: context.taskId,
              agentName: agentContext.agent.Name,
              nodeId: agentContext.context.taskId,
              type: 'finish',
              finishReason: (chunk.finishReason as FinishReason) || 'stop',
              usage: chunk.usage || {
                promptTokens: 0,
                completionTokens: 0,
                totalTokens: 0,
              },
            },
            agentContext
          )

          if (chunk.finishReason === 'length' && messages.length >= 5 && !noCompress && retryNum < config.maxRetryNum) {
            await memory.compressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
            return callAgentLLM(agentContext, rlm, messages, tools, noCompress, toolChoice, ++retryNum, streamCallback)
          }
          break
        }
        case 'error': {
          Log.error(`${agentContext.agent.Name} agent error: `, chunk.error)
          await streamCallback.onMessage(
            {
              taskId: context.taskId,
              agentName: agentContext.agent.Name,
              nodeId: agentContext.context.taskId,
              type: 'error',
              error: chunk.error,
            },
            agentContext
          )
          throw new Error('LLM Error: ' + chunk.error)
        }
      }
    }
  } catch (e: unknown) {
    await context.checkAborted()

    // Use enhanced error handling for retry decisions
    if (retryNum < config.maxRetryNum) {
      // Use exponential backoff with jitter for stream errors
      const baseDelay = 200
      const delay = Math.min(baseDelay * Math.pow(2, retryNum), 5000) + Math.random() * 1000
      await sleep(delay)
      return callAgentLLM(agentContext, rlm, messages, tools, noCompress, toolChoice, ++retryNum, streamCallback)
    }
    throw e
  } finally {
    reader.releaseLock()
    context.currentStepControllers.delete(stepController)
  }

  // Store result in chain for single agent mode
  agentContext.context.chain.planResult = streamText

  // Return results in native format
  const results: Array<{ type: 'text'; text: string } | NativeLLMToolCall> = []
  if (streamText) {
    results.push({ type: 'text', text: streamText })
  }
  results.push(...toolCalls)

  return results
}

function appendUserConversation(agentContext: AgentContext, messages: NativeLLMMessage[]) {
  const userPrompts = agentContext.context.conversation
    .splice(0, agentContext.context.conversation.length)
    .filter(s => !!s)
  if (userPrompts.length > 0) {
    const prompt =
      'The user is intervening in the current task, please replan and execute according to the following instructions:\n' +
      userPrompts.map(s => `- ${s.trim()}`).join('\n')
    messages.push({
      role: 'user',
      content: prompt,
    })
  }
}
