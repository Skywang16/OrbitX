/**
 * 本地 Eko 类型定义与导出
 */

import type { AgentContext } from '@eko-ai/eko'

// 从 Eko 包导出工具相关类型，供本地代码统一引用
export type { Tool, ToolResult } from '@eko-ai/eko/types'

/**
 * 流式消息类型（与侧边栏使用一致）
 */
export interface StreamMessage {
  type:
    | 'tool_use'
    | 'tool_result'
    | 'workflow'
    | 'text'
    | 'thinking'
    | 'agent_start'
    | 'agent_result'
    | 'tool_streaming'
    | 'tool_running'
    | 'file'
    | 'error'
    | 'finish'
  toolName?: string
  params?: Record<string, unknown>
  toolResult?: unknown
  thought?: string
  text?: string
  streamId?: string
  streamDone?: boolean
  workflow?: {
    thought?: string
  }
  // 新增字段支持更多回调类型
  agentName?: string
  agentResult?: unknown
  toolStreaming?: {
    paramName?: string
    paramValue?: unknown
    isComplete?: boolean
  }
  fileData?: {
    fileName?: string
    filePath?: string
    content?: string
    mimeType?: string
  }
  error?: {
    message?: string
    code?: string
    details?: unknown
  }
  finish?: {
    tokenUsage?: {
      promptTokens?: number
      completionTokens?: number
      totalTokens?: number
    }
    duration?: number
    status?: 'success' | 'error' | 'cancelled'
  }
}

/**
 * Eko 回调接口（与 Eko 的 StreamCallback & HumanCallback 对齐）
 */
export interface TerminalCallback {
  onMessage: (message: StreamCallbackMessage, agentContext?: AgentContext) => Promise<void>
  onHumanConfirm: (agentContext: AgentContext, prompt: string) => Promise<boolean>
  onHumanInput: (agentContext: AgentContext, prompt: string) => Promise<string>
  onHumanSelect: (agentContext: AgentContext, prompt: string, options: readonly string[]) => Promise<string[]>
  onHumanHelp: (agentContext: AgentContext, helpType: string, prompt: string) => Promise<boolean>
  onCommandConfirm: (agentContext: AgentContext, command: string) => Promise<boolean>
  onFileSelect: (agentContext: AgentContext, prompt: string, directory?: string) => Promise<string>
  onPathInput: (agentContext: AgentContext, prompt: string, defaultPath?: string) => Promise<string>
}

// Import StreamCallbackMessage from the Eko package to avoid type mismatches
import type { StreamCallbackMessage } from '@eko-ai/eko/types'
export type { StreamCallbackMessage }

/**
 * 终端 Agent 配置
 */
export interface TerminalAgentConfig {
  name: string
  description: string
  defaultTerminalId?: number
  defaultWorkingDirectory?: string
  safeMode?: boolean
  allowedCommands?: string[]
  blockedCommands?: string[]
}

/**
 * Eko 实例配置
 */
export interface CodeAgentConfig {
  name: string
  description: string
  defaultWorkingDirectory?: string
  safeMode: boolean
  supportedLanguages: string[]
  codeStyle: {
    indentSize: number
    indentType: 'spaces' | 'tabs'
    maxLineLength: number
    insertFinalNewline: boolean
    trimTrailingWhitespace: boolean
  }
  enabledFeatures: {
    codeGeneration: boolean
    codeAnalysis: boolean
    refactoring: boolean
    formatting: boolean
    linting: boolean
    testing: boolean
    documentation: boolean
  }
}

export interface EkoInstanceConfig {
  callback?: TerminalCallback
  agentConfig?: Partial<TerminalAgentConfig>
  codeAgentConfig?: Partial<TerminalAgentConfig>
  debug?: boolean
}

/**
 * 运行选项
 */
export interface EkoRunOptions {
  terminalId?: number
  workingDirectory?: string
}

/**
 * 运行结果
 */
export interface EkoRunResult {
  result: string
  duration: number
  success: boolean
  error?: string
}
