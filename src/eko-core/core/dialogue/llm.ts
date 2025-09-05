import { DialogueCallback, EkoMessageAssistantPart, EkoMessageToolPart, EkoMessageUserPart } from '../../types'
import {
  NativeLLMMessage,
  NativeLLMMessagePart,
  NativeLLMTool,
  NativeLLMStreamChunk,
  LLMRequest,
} from '../../types/llm.types'
import config from '../../config'
import Log from '../../common/log'
import { RetryLanguageModel } from '../../llm'
import { sleep, uuidv4 } from '../../common/utils'

export async function callChatLLM(
  messageId: string,
  rlm: RetryLanguageModel,
  messages: NativeLLMMessage[],
  tools: NativeLLMTool[],
  toolChoice?: string,
  retryNum: number = 0,
  callback?: DialogueCallback,
  signal?: AbortSignal
): Promise<Array<NativeLLMMessagePart>> {
  const streamCallback = callback?.chatCallback || {
    onMessage: async () => {},
  }
  const request: LLMRequest = {
    tools: tools,
    toolChoice,
    messages: messages,
    abortSignal: signal,
  }
  const streamResult = await rlm.callStream(request)
  let streamText = ''
  let textStreamId = uuidv4()
  let textStreamDone = false
  const toolParts: NativeLLMMessagePart[] = []
  const reader = streamResult.stream.getReader()
  try {
    while (true) {
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
            await streamCallback.onMessage({
              type: 'text',
              streamId: textStreamId,
              streamDone: false,
              text: streamText,
            })
          }

          // Handle tool calls
          if (chunk.toolCalls && chunk.toolCalls.length > 0) {
            if (!textStreamDone && streamText) {
              textStreamDone = true
              await streamCallback.onMessage({
                type: 'text',
                streamId: textStreamId,
                streamDone: true,
                text: streamText,
              })
            }

            for (const toolCall of chunk.toolCalls) {
              await streamCallback.onMessage({
                type: 'tool_use',
                toolId: toolCall.id,
                toolName: toolCall.name,
                params: toolCall.arguments,
              })

              // Add to tool parts
              toolParts.push({
                type: 'tool-call',
                toolCallId: toolCall.id,
                toolName: toolCall.name,
                args: toolCall.arguments,
              })
            }
          }
          break
        }
        case 'error': {
          Log.error(`chatLLM error: `, chunk)
          await streamCallback.onMessage({
            type: 'error',
            error: chunk.error,
          })
          throw new Error('LLM Error: ' + chunk.error)
        }
        case 'finish': {
          if (!textStreamDone && streamText) {
            textStreamDone = true
            await streamCallback.onMessage({
              type: 'text',
              streamId: textStreamId,
              streamDone: true,
              text: streamText,
            })
          }

          await streamCallback.onMessage({
            type: 'finish',
            finishReason: chunk.finishReason as any,
            usage: {
              promptTokens: chunk.usage?.promptTokens || 0,
              completionTokens: chunk.usage?.completionTokens || 0,
              totalTokens: chunk.usage?.totalTokens || 0,
            },
          })
          break
        }
      }
    }
  } catch (e: unknown) {
    if (retryNum < config.maxRetryNum) {
      await sleep(200 * (retryNum + 1) * (retryNum + 1))
      return callChatLLM(messageId, rlm, messages, tools, toolChoice, ++retryNum, callback, signal)
    }
    throw e
  } finally {
    reader.releaseLock()
  }

  // Return text and tool parts
  const result: NativeLLMMessagePart[] = []
  if (streamText) {
    result.push({ type: 'text', text: streamText })
  }
  result.push(...toolParts)
  return result
}

export function convertAssistantToolResults(results: NativeLLMMessagePart[]): EkoMessageAssistantPart[] {
  return results.map(part => {
    if (part.type == 'text') {
      return {
        type: 'text',
        text: part.text || '',
      }
    } else if (part.type == 'tool-call') {
      return {
        type: 'tool-call',
        toolCallId: part.toolCallId || '',
        toolName: part.toolName || '',
        args: (part.args || {}) as Record<string, unknown>,
      }
    }
    return part as EkoMessageAssistantPart
  })
}

export function convertToolResults(toolResults: NativeLLMMessagePart[]): EkoMessageToolPart[] {
  return toolResults.map(part => {
    if (part.type !== 'tool-result') {
      throw new Error('Expected tool-result part')
    }

    return {
      type: 'tool-result',
      toolCallId: part.toolCallId || '',
      toolName: part.toolName || '',
      result: part.result || '',
    }
  })
}

export function convertUserContent(content: NativeLLMMessagePart[]): EkoMessageUserPart[] {
  return content.map(part => {
    if (part.type == 'text') {
      return {
        type: 'text',
        text: part.text || '',
      }
    } else if (part.type == 'file') {
      return {
        type: 'file',
        mimeType: part.mimeType || 'application/octet-stream',
        data: part.data || '',
      }
    }
    return part as EkoMessageUserPart
  })
}
