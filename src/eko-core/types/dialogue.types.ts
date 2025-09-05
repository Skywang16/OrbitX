import { ToolResult } from './tools.types'
import { EkoConfig, HumanCallback, StreamCallback } from './core.types'
import { NativeLLMToolCall, FinishReason, JSONSchema7 } from './llm.types'

export type EkoMessage =
  | {
      id: string
      role: 'user'
      timestamp: number
      content: string | EkoMessageUserPart[]
    }
  | {
      id: string
      role: 'assistant'
      timestamp: number
      content: EkoMessageAssistantPart[]
    }
  | {
      id: string
      role: 'tool'
      timestamp: number
      content: EkoMessageToolPart[]
    }

export type EkoMessageUserPart =
  | {
      type: 'text'
      text: string
    }
  | {
      type: 'file'
      mimeType: string
      data: string // base64 / URL
    }

export type EkoMessageAssistantPart =
  | {
      type: 'text'
      text: string
    }
  | {
      type: 'reasoning'
      text: string
    }
  | {
      type: 'tool-call'
      toolCallId: string
      toolName: string
      args: Record<string, unknown>
    }

export type EkoMessageToolPart = {
  type: 'tool-result'
  toolCallId: string
  toolName: string
  result: string | Record<string, unknown>
}

export interface DialogueTool {
  readonly name: string
  readonly description?: string
  readonly parameters: JSONSchema7
  execute: (args: Record<string, unknown>, toolCall: NativeLLMToolCall) => Promise<ToolResult>
}

export type EkoDialogueConfig = Omit<EkoConfig, 'callback'> & {
  chatLlms?: string[]
  segmentedExecution?: boolean
}

export type DialogueParams = {
  user: string | EkoMessageUserPart[]
  callback?: DialogueCallback
  messageId?: string
  signal?: AbortSignal
}

export type DialogueCallback = {
  chatCallback?: {
    onMessage: (message: ChatStreamCallbackMessage) => Promise<void>
  }
  taskCallback?: StreamCallback & HumanCallback
}

export type ChatStreamCallbackMessage =
  | {
      type: 'text' | 'thinking'
      streamId: string
      streamDone: boolean
      text: string
    }
  | {
      type: 'tool_streaming'
      toolName: string
      toolId: string
      paramsText: string
    }
  | {
      type: 'tool_use'
      toolName: string
      toolId: string
      params: Record<string, unknown>
    }
  | {
      type: 'tool_running'
      toolName: string
      toolId: string
      text: string
      streamId: string
      streamDone: boolean
    }
  | {
      type: 'tool_result'
      toolName: string
      toolId: string
      params: Record<string, unknown>
      toolResult: ToolResult
    }
  | {
      type: 'error'
      error: unknown
    }
  | {
      type: 'finish'
      finishReason: FinishReason
      usage: {
        promptTokens: number
        completionTokens: number
        totalTokens: number
      }
    }
