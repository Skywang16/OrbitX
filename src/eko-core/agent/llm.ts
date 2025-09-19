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
  NativeLLMUsage,
} from '../types'
import { convertTools, getTool, convertToolResult } from '../llm/conversion-utils'

// Export conversion utilities from llm module
export { convertTools, getTool, convertToolResult }

export interface AgentLLMCallResult {
  rawText: string
  thinkingText: string
  responseText: string
  toolCalls: NativeLLMToolCall[]
  finishReason?: FinishReason
  usage?: NativeLLMUsage
}

export function removeDuplicateToolUse(toolCalls: NativeLLMToolCall[]): NativeLLMToolCall[] {
  if (toolCalls.length <= 1) {
    return toolCalls
  }

  const deduped: NativeLLMToolCall[] = []
  const seen: string[] = []

  for (const toolCall of toolCalls) {
    const key = `${toolCall.name}:${JSON.stringify(toolCall.arguments || {})}`
    if (!seen.includes(key)) {
      deduped.push(toolCall)
      seen.push(key)
    }
  }

  return deduped
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
): Promise<AgentLLMCallResult> {
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
  const toolCalls: NativeLLMToolCall[] = []
  const reader = result.stream.getReader()
  let finishReason: FinishReason | undefined
  let finishUsage: NativeLLMUsage | undefined
  // Use stable node and stream IDs for streaming updates
  const textNodeId = agentContext.context.currentNodeId || generateNodeId(context.taskId, 'execution')
  const textStreamId = uuidv4()
  const thinkingStreamId = uuidv4()
  let sawThinkingTag = false
  let lastThinkingSent = ''
  let lastVisibleSent = ''

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
            // Detect whether thinking tag appears in the stream
            if (!sawThinkingTag && streamText.includes('<thinking')) {
              sawThinkingTag = true
            }

            // Stream incremental thinking/text updates using stable streamIds
            const { thinking, visible, hasOpenThinking } = splitThinkingSections(streamText)
            const thinkingTrim = thinking.trim()
            const canSendVisible =
              !!visible && !visible.includes('<thinking') && !hasOpenThinking && visible.trim().length > 0

            if (sawThinkingTag) {
              // Maintain order: send thinking first; if no thinking content yet, hold text updates
              if (thinkingTrim.length > 0 && thinkingTrim !== lastThinkingSent) {
                lastThinkingSent = thinkingTrim
                await streamCallback.onMessage(
                  {
                    taskId: context.taskId,
                    agentName: agentContext.agent.Name,
                    nodeId: textNodeId,
                    type: 'thinking',
                    streamId: thinkingStreamId,
                    streamDone: false,
                    text: thinkingTrim,
                  },
                  agentContext
                )
              }
              if (canSendVisible && visible !== lastVisibleSent && lastThinkingSent.trim().length > 0) {
                lastVisibleSent = visible
                await streamCallback.onMessage(
                  {
                    taskId: context.taskId,
                    agentName: agentContext.agent.Name,
                    nodeId: textNodeId,
                    type: 'text',
                    streamId: textStreamId,
                    streamDone: false,
                    text: visible,
                  },
                  agentContext
                )
              }
            } else {
              // No thinking tag seen: stream text normally
              if (canSendVisible && visible !== lastVisibleSent) {
                lastVisibleSent = visible
                await streamCallback.onMessage(
                  {
                    taskId: context.taskId,
                    agentName: agentContext.agent.Name,
                    nodeId: textNodeId,
                    type: 'text',
                    streamId: textStreamId,
                    streamDone: false,
                    text: visible,
                  },
                  agentContext
                )
              }
            }
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
                  nodeId: textNodeId,
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
          finishReason = (chunk.finishReason as FinishReason) || 'stop'
          finishUsage = chunk.usage
          break
        }
        case 'error': {
          const errorMsg = typeof chunk.error === 'string' ? chunk.error : String(chunk.error || 'Unknown error')
          Log.error(`${agentContext.agent.Name} agent error: `, errorMsg)
          await streamCallback.onMessage(
            {
              taskId: context.taskId,
              agentName: agentContext.agent.Name,
              nodeId: textNodeId,
              type: 'error',
              error: errorMsg,
            },
            agentContext
          )
          throw new Error('LLM Error: ' + errorMsg)
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

  // Final thinking/text processing: close both streams using the latest split
  const { thinking: finalThinking, visible: finalVisible } = splitThinkingSections(streamText)
  const finalThought = finalThinking.trim()
  const finalText = finalVisible.trim()
  if (sawThinkingTag && (finalThought || lastThinkingSent)) {
    await streamCallback.onMessage(
      {
        taskId: context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: textNodeId,
        type: 'thinking',
        streamId: thinkingStreamId,
        streamDone: true,
        text: finalThought || lastThinkingSent,
      },
      agentContext
    )
  }
  if (finalText) {
    await streamCallback.onMessage(
      {
        taskId: context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: textNodeId,
        type: 'text',
        streamId: textStreamId,
        streamDone: true,
        text: finalText,
      },
      agentContext
    )
  }

  if (finishReason) {
    const usage = finishUsage || {
      promptTokens: 0,
      completionTokens: 0,
      totalTokens: 0,
    }
    await streamCallback.onMessage(
      {
        taskId: context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: textNodeId,
        type: 'finish',
        finishReason,
        usage,
      },
      agentContext
    )

    if (finishReason === 'length' && messages.length >= 5 && !noCompress && retryNum < config.maxRetryNum) {
      await memory.compressAgentMessages(agentContext, rlm, messages, tools, callAgentLLM)
      return callAgentLLM(agentContext, rlm, messages, tools, noCompress, toolChoice, ++retryNum, streamCallback)
    }
  }

  // Store result in chain for single agent mode
  agentContext.context.chain.planResult = streamText

  const dedupedToolCalls = removeDuplicateToolUse(toolCalls)

  const { thinking: retThinking, visible: retVisible } = splitThinkingSections(streamText)
  return {
    rawText: streamText,
    thinkingText: retThinking.trim(),
    responseText: retVisible.trim(),
    toolCalls: dedupedToolCalls,
    finishReason,
    usage: finishUsage,
  }
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

// Streaming-aware thinking/text splitter
// - thinking: all closed <thinking>...</thinking> blocks + any open partial after the last <thinking>
// - visible: raw text with closed blocks removed and any trailing open or incomplete <thinking left out
// - hasOpenThinking: whether the stream is currently inside an open or incomplete <thinking> block
function splitThinkingSections(raw: string): { thinking: string; visible: string; hasOpenThinking: boolean } {
  if (!raw) {
    return { thinking: '', visible: '', hasOpenThinking: false }
  }

  // Collect closed thinking blocks
  const closedRegex = /<thinking>([\s\S]*?)<\/thinking>/gi
  const thinkingParts: string[] = []
  let m: RegExpExecArray | null
  while ((m = closedRegex.exec(raw)) !== null) {
    thinkingParts.push(m[1])
  }

  // Remove closed blocks for further processing
  let working = raw.replace(closedRegex, '')

  // Detect open or incomplete thinking markers on working
  let hasOpenThinking = false
  const lastThinkingIdx = working.lastIndexOf('<thinking')
  let partial = ''
  let visible = working

  if (lastThinkingIdx !== -1) {
    const endBracket = working.indexOf('>', lastThinkingIdx)
    if (endBracket === -1) {
      // Incomplete tag, treat as open-thinking-in-progress; drop it from visible
      hasOpenThinking = true
      visible = working.substring(0, lastThinkingIdx)
    } else {
      // Complete <thinking> without a matching close in 'working' (since closed ones were removed)
      hasOpenThinking = true
      const lastOpenIdx = working.lastIndexOf('<thinking>')
      if (lastOpenIdx !== -1) {
        visible = working.substring(0, lastOpenIdx)
        partial = working.substring(lastOpenIdx + '<thinking>'.length)
      }
    }
  }

  const thinkingAll = [thinkingParts.join('\n'), partial].filter(Boolean).join('\n').trim()
  return { thinking: thinkingAll, visible, hasOpenThinking }
}
