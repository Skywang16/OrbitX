import type { JSONSchema7 as LibJSONSchema7 } from 'json-schema'
export type JSONSchema7 = LibJSONSchema7 | boolean

// Core message types
export interface NativeLLMMessage {
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string | NativeLLMMessagePart[]
}

export interface NativeLLMMessagePart {
  type: 'text' | 'file' | 'tool-call' | 'tool-result'
  text?: string
  mimeType?: string
  data?: string
  toolCallId?: string
  toolName?: string
  args?: Record<string, unknown>
  result?: string | Record<string, unknown>
}

// Tool types
export interface NativeLLMTool {
  name: string
  description: string
  parameters: JSONSchema7
}

export interface NativeLLMToolCall {
  id: string
  name: string
  arguments: Record<string, unknown>
}

// Request and response types
export interface NativeLLMRequest {
  model: string
  messages: NativeLLMMessage[]
  temperature?: number
  maxTokens?: number
  tools?: NativeLLMTool[]
  toolChoice?: string
  stream: boolean
  abortSignal?: AbortSignal
}

export interface NativeLLMResponse {
  content: string
  finishReason: string
  toolCalls?: NativeLLMToolCall[]
  usage?: NativeLLMUsage
}

export interface NativeLLMUsage {
  promptTokens: number
  completionTokens: number
  totalTokens: number
}

// Stream types
export type NativeLLMStreamChunk =
  | { type: 'delta'; content?: string; toolCalls?: NativeLLMToolCall[] }
  | { type: 'finish'; finishReason: string; usage?: NativeLLMUsage }
  | { type: 'error'; error: string }

// Simplified configuration types
export type LLMConfig = {
  modelId: string
  temperature?: number
  maxTokens?: number
}

export type LLMs = {
  default: LLMConfig
  [key: string]: LLMConfig
}

export type FinishReason = 'stop' | 'length' | 'tool_calls' | 'content_filter'

// Generate result types
export type GenerateResult = {
  modelId: string
  content: string
  finishReason: FinishReason
  usage: NativeLLMUsage
  toolCalls?: NativeLLMToolCall[]
}

// Stream result types
export type StreamResult = {
  modelId: string
  stream: ReadableStream<NativeLLMStreamChunk>
}

// Request type for compatibility
export type LLMRequest = {
  maxTokens?: number
  messages: NativeLLMMessage[]
  toolChoice?: string
  tools?: NativeLLMTool[]
  temperature?: number
  stopSequences?: string[]
  abortSignal?: AbortSignal
}

// Eko memory message type (replaces old dialogue message types)
export interface EkoMessage {
  id: string
  role: 'user' | 'assistant' | 'tool'
  timestamp: number
  content: string | NativeLLMMessagePart[]
}
