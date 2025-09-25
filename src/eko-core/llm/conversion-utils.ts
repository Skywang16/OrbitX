import { NativeLLMMessage, NativeLLMMessagePart, NativeLLMTool, NativeLLMToolCall } from '../types/llm.types'
import { Tool, ToolResult } from '../types'

/**
 * Convert tools to native format
 */
export function convertTools(tools: Tool[]): NativeLLMTool[] {
  return tools.map(tool => ({
    name: tool.name,
    description: tool.description || '',
    parameters: tool.parameters,
  }))
}

/**
 * Get tool by name from tool array
 */
export function getTool<T extends Tool>(tools: T[], name: string): T | null {
  for (let i = 0; i < tools.length; i++) {
    if (tools[i].name === name) {
      return tools[i]
    }
  }
  return null
}

/**
 * Convert tool result to native message part
 */
export function convertToolResult(toolCall: NativeLLMToolCall, toolResult: ToolResult): NativeLLMMessagePart {
  let result: string | Record<string, unknown>

  if (toolResult.content.length === 1 && toolResult.content[0].type === 'text') {
    let text = toolResult.content[0].text

    if (toolResult.isError === true && !text.startsWith('Error')) {
      text = 'Error: ' + text
    } else if (toolResult.isError !== true && text.length === 0) {
      text = 'Successful'
    }

    // Try to parse as JSON
    if (text && ((text.startsWith('{') && text.endsWith('}')) || (text.startsWith('[') && text.endsWith(']')))) {
      try {
        result = JSON.parse(text)
      } catch (e) {
        result = text
      }
    } else {
      result = text
    }
  } else {
    // Handle multi-part content
    const parts: unknown[] = []
    for (const content of toolResult.content) {
      if (content.type === 'text') {
        parts.push({ type: 'text', text: content.text })
      } else {
        // For non-text content, convert to text description
        parts.push({
          type: 'text',
          text: `[${content.type}: ${content.mimeType || 'unknown'}]`,
        })
      }
    }
    result = { type: 'content', value: parts }
  }

  return {
    type: 'tool-result',
    toolCallId: toolCall.id,
    toolName: toolCall.name,
    result,
  }
}

/**
 * Create a text message
 */
export function createTextMessage(role: 'system' | 'user' | 'assistant', text: string): NativeLLMMessage {
  return {
    role,
    content: text,
  }
}

/**
 * Create a tool call message
 */
export function createToolCallMessage(toolCalls: NativeLLMToolCall[]): NativeLLMMessage {
  const parts: NativeLLMMessagePart[] = toolCalls.map(toolCall => ({
    type: 'tool-call',
    toolCallId: toolCall.id,
    toolName: toolCall.name,
    args: toolCall.arguments as Record<string, unknown>,
  }))

  return {
    role: 'assistant',
    content: parts,
  }
}

/**
 * Create a tool result message
 */
export function createToolResultMessage(toolCall: NativeLLMToolCall, toolResult: ToolResult): NativeLLMMessage {
  return {
    role: 'tool',
    content: [convertToolResult(toolCall, toolResult)],
  }
}

/**
 * Convert file data to native message part
 */
export function createFileMessagePart(data: string, mimeType: string): NativeLLMMessagePart {
  return {
    type: 'file',
    data,
    mimeType,
  }
}

/**
 * Create a multimodal message with text and files
 */
export function createMultimodalMessage(
  role: 'user' | 'assistant',
  textContent: string,
  files?: Array<{ data: string; mimeType: string }>
): NativeLLMMessage {
  const parts: NativeLLMMessagePart[] = [{ type: 'text', text: textContent }]

  if (files) {
    parts.push(...files.map(file => createFileMessagePart(file.data, file.mimeType)))
  }

  return {
    role,
    content: parts,
  }
}
