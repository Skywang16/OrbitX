/**
 * LLM相关类型定义
 */

import type { ToolDefinition } from './tool'

/**
 * LLM调用选项
 */
export interface LLMCallOptions {
  model?: string
  temperature?: number
  maxTokens?: number
  stream?: boolean
  timeout?: number
  systemPrompt?: string
  tools?: ToolDefinition[]
  toolChoice?: 'auto' | 'none' | { type: 'function'; function: { name: string } }
}

/**
 * LLM响应
 */
export interface LLMResponse {
  content: string
  finishReason?: 'stop' | 'length' | 'tool_calls' | 'content_filter'
  toolCalls?: Array<{
    id: string
    type: 'function'
    function: {
      name: string
      arguments: string
    }
  }>
  usage?: {
    promptTokens: number
    completionTokens: number
    totalTokens: number
  }
  metadata?: Record<string, unknown>
}

/**
 * LLM流式响应块
 */
export interface LLMStreamChunk {
  content?: string
  toolCalls?: Array<{
    id: string
    type: 'function'
    function: {
      name: string
      arguments: string
    }
  }>
  finishReason?: string
  metadata?: Record<string, unknown>
}

/**
 * LLM接口
 */
export interface LLMProvider {
  name: string
  call(prompt: string, options?: LLMCallOptions): Promise<LLMResponse>
  stream(prompt: string, options?: LLMCallOptions): AsyncIterable<LLMStreamChunk>
  isAvailable(): Promise<boolean>
}
