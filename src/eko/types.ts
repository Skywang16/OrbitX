/**
 * 本地 Eko 类型定义与导出
 */

import type { AgentContext } from '@/eko-core'

// 从 Eko 包导出工具相关类型，供本地代码统一引用
export type { Tool, ToolResult, Task } from '@/eko-core/types'

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
import type { StreamCallbackMessage } from '@/eko-core/types'
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

export interface EkoInstanceConfig {
  callback?: TerminalCallback
  agentConfig?: Partial<TerminalAgentConfig>
  debug?: boolean
  selectedModelId?: string | null
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
