import { Tool, ToolSchema, NativeLLMTool, NativeLLMToolCall, NativeLLMMessagePart, NativeLLMMessage } from '../types'
import { ToolResult } from '../types/tools.types'

export function sleep(time: number): Promise<void> {
  return new Promise(resolve => setTimeout(() => resolve(), time))
}

export function uuidv4(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}

export function call_timeout<T>(
  fun: () => Promise<T>,
  timeout: number,
  error_callback?: (e: string) => void
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error('Timeout'))
      if (error_callback) error_callback('Timeout')
    }, timeout)

    fun()
      .then((result: T) => {
        clearTimeout(timer)
        resolve(result)
      })
      .catch(e => {
        clearTimeout(timer)
        reject(e)
        if (error_callback) error_callback(String(e))
      })
  })
}

// Native tool conversion functions
export function convertToolSchema(tool: ToolSchema): NativeLLMTool {
  if ('function' in tool) {
    return {
      name: tool.function.name,
      description: tool.function.description || '',
      parameters: tool.function.parameters,
    }
  } else if ('input_schema' in tool) {
    return {
      name: tool.name,
      description: tool.description || '',
      parameters: tool.input_schema,
    }
  } else if ('inputSchema' in tool) {
    return {
      name: tool.name,
      description: tool.description || '',
      parameters: tool.inputSchema,
    }
  } else {
    return {
      name: tool.name,
      description: tool.description || '',
      parameters: tool.parameters,
    }
  }
}

export function convertTools(tools: Tool[]): NativeLLMTool[] {
  return tools.map(tool => ({
    name: tool.name,
    description: tool.description || '',
    parameters: tool.parameters,
  }))
}

export function convertToolResult(toolCall: NativeLLMToolCall, toolResult: ToolResult): NativeLLMMessagePart {
  // Convert ToolResult content to a simple string or object format
  let result: string | Record<string, unknown>

  if (toolResult.content.length === 1 && toolResult.content[0].type === 'text') {
    result = toolResult.content[0].text
  } else {
    // Convert complex content to a structured object
    result = {
      content: toolResult.content,
      isError: toolResult.isError,
      extInfo: toolResult.extInfo,
    }
  }

  return {
    type: 'tool-result',
    toolCallId: toolCall.id,
    toolName: toolCall.name,
    result,
  }
}

export function toImage(mediaData: string): Uint8Array | string | URL {
  return toFile(mediaData)
}

export function toFile(mediaData: string, type: 'base64|url' | 'binary|url' = 'base64|url'): Uint8Array | string | URL {
  if (mediaData.startsWith('http://') || mediaData.startsWith('https://')) {
    return new URL(mediaData)
  } else if (mediaData.startsWith('//') && mediaData.indexOf('.') > 0 && mediaData.length < 1000) {
    return new URL('https:' + mediaData)
  }
  if (mediaData.startsWith('data:')) {
    mediaData = mediaData.substring(mediaData.indexOf(',') + 1)
  }
  if (type === 'binary|url') {
    // @ts-ignore
    if (typeof Buffer != 'undefined') {
      // @ts-ignore
      const buffer = Buffer.from(mediaData, 'base64')
      return new Uint8Array(buffer)
    } else {
      const binaryString = atob(mediaData)
      const fileData = new Uint8Array(binaryString.length)
      for (let i = 0; i < binaryString.length; i++) {
        fileData[i] = binaryString.charCodeAt(i)
      }
      return fileData
    }
  } else {
    return mediaData
  }
}

export function getMimeType(data: string): string {
  let mediaType = 'image/png'
  if (data.startsWith('data:')) {
    mediaType = data.split(';')[0].split(':')[1]
  } else if (data.indexOf('.') > -1) {
    if (data.indexOf('.png') > -1) {
      mediaType = 'image/png'
    } else if (data.indexOf('.jpg') > -1 || data.indexOf('.jpeg') > -1) {
      mediaType = 'image/jpeg'
    } else if (data.indexOf('.pdf') > -1) {
      mediaType = 'application/pdf'
    } else if (data.indexOf('.docx') > -1) {
      mediaType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
    } else if (data.indexOf('.xlsx') > -1) {
      mediaType = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
    } else if (data.indexOf('.pptx') > -1) {
      mediaType = 'application/vnd.openxmlformats-officedocument.presentationml.presentation'
    } else if (data.indexOf('.txt') > -1) {
      mediaType = 'text/plain'
    } else if (data.indexOf('.md') > -1) {
      mediaType = 'text/markdown'
    } else if (data.indexOf('.json') > -1) {
      mediaType = 'application/json'
    } else if (data.indexOf('.xml') > -1) {
      mediaType = 'application/xml'
    } else if (data.indexOf('.csv') > -1) {
      mediaType = 'text/csv'
    }
  }
  return mediaType
}

export function mergeTools<T extends Tool | NativeLLMTool>(tools1: T[], tools2: T[]): T[] {
  let tools: T[] = []
  let toolMap2 = tools2.reduce(
    (map, tool) => {
      map[tool.name] = tool
      return map
    },
    {} as Record<string, T>
  )
  let names = []
  for (let i = 0; i < tools1.length; i++) {
    let tool1 = tools1[i]
    let tool2 = toolMap2[tool1.name]
    if (tool2) {
      tools.push(tool2)
      delete toolMap2[tool1.name]
    } else {
      tools.push(tool1)
    }
  }
  for (let i = 0; i < tools2.length; i++) {
    let tool2 = tools2[i]
    if (toolMap2[tool2.name] && names.indexOf(tool2.name) === -1) {
      tools.push(tool2)
      names.push(tool2.name)
    }
  }
  return tools
}

// Additional utility functions for native backend
export function createTextMessage(role: 'system' | 'user' | 'assistant', content: string): NativeLLMMessage {
  return { role, content }
}

export function createToolCallMessage(toolCalls: NativeLLMToolCall[]): NativeLLMMessage {
  return {
    role: 'assistant',
    content: toolCalls.map(call => ({
      type: 'tool-call' as const,
      toolCallId: call.id,
      toolName: call.name,
      args: call.arguments,
    })),
  }
}

export function createToolResultMessage(toolCall: NativeLLMToolCall, result: ToolResult): NativeLLMMessage {
  return {
    role: 'tool',
    content: [convertToolResult(toolCall, result)],
  }
}

export function isTextMessage(message: NativeLLMMessage): boolean {
  return typeof message.content === 'string'
}

export function isMultiPartMessage(message: NativeLLMMessage): boolean {
  return Array.isArray(message.content)
}

export function extractTextFromMessage(message: NativeLLMMessage): string {
  if (typeof message.content === 'string') {
    return message.content
  }

  return message.content
    .filter(part => part.type === 'text')
    .map(part => part.text || '')
    .join('')
}

export function extractToolCallsFromMessage(message: NativeLLMMessage): NativeLLMToolCall[] {
  if (typeof message.content === 'string') {
    return []
  }

  return message.content
    .filter(part => part.type === 'tool-call')
    .map(part => ({
      id: part.toolCallId || '',
      name: part.toolName || '',
      arguments: part.args || {},
    }))
}

// mergeAgents removed - single agent mode only

export function sub(str: string, maxLength: number, appendPoint: boolean = true): string {
  if (!str) {
    return ''
  }
  if (str.length > maxLength) {
    return str.substring(0, maxLength) + (appendPoint ? '...' : '')
  }
  return str
}

export function fixXmlTag(code: string) {
  code = code.trim()
  if (code.endsWith('<')) {
    code = code.substring(0, code.length - 1)
  }
  if (code.indexOf('&') > -1) {
    code = code.replace(/&(?![a-zA-Z0-9#]+;)/g, '&amp;')
  }
  function fixDoubleChar(code: string) {
    const stack: string[] = []
    for (let i = 0; i < code.length; i++) {
      let s = code[i]
      if (s === '<') {
        stack.push('>')
      } else if (s === '>') {
        stack.pop()
      } else if (s === '"') {
        if (stack[stack.length - 1] === '"') {
          stack.pop()
        } else {
          stack.push('"')
        }
      }
    }
    const missingParts = []
    while (stack.length > 0) {
      missingParts.push(stack.pop())
    }
    return code + missingParts.join('')
  }
  let eIdx = code.lastIndexOf(' ')
  let endStr = eIdx > -1 ? code.substring(eIdx + 1) : ''
  if (code.endsWith('=')) {
    code += '""'
  } else if (
    endStr == 'name' ||
    endStr == 'id' ||
    endStr == 'depen' ||
    endStr == 'depends' ||
    endStr == 'dependsOn' ||
    endStr == 'items' ||
    endStr == 'event' ||
    endStr == 'loop'
  ) {
    let idx1 = code.lastIndexOf('>')
    let idx2 = code.lastIndexOf('<')
    if (idx1 < idx2 && code.lastIndexOf(' ') > idx2) {
      code += '=""'
    }
  }
  code = fixDoubleChar(code)
  const stack: string[] = []
  function isSelfClosing(tag: string) {
    return tag.endsWith('/>')
  }
  for (let i = 0; i < code.length; i++) {
    let s = code[i]
    if (s === '<') {
      const isEndTag = code[i + 1] === '/'
      let endIndex = code.indexOf('>', i)
      let tagContent = code.slice(i, endIndex + 1)
      if (isSelfClosing(tagContent)) {
        // no-op for self-closing tags
      } else if (isEndTag) {
        stack.pop()
      } else {
        stack.push(tagContent)
      }
      if (endIndex == -1) {
        break
      }
      i = endIndex
    }
  }
  const missingParts = []
  while (stack.length > 0) {
    const top = stack.pop() as string
    if (top.startsWith('<')) {
      let arr = top.match(/<(\w+)/) as string[]
      if (arr) {
        const tagName = arr[1]
        missingParts.push(`</${tagName}>`)
      }
    } else {
      missingParts.push(top)
    }
  }
  let completedCode = code + missingParts.join('')
  return completedCode
}
