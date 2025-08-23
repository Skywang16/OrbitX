/**
 * 终端模块相关的类型定义
 */

// ===== 终端基本操作类型 =====

export interface TerminalCreateOptions {
  rows: number
  cols: number
  cwd?: string
}

export interface TerminalWriteOptions {
  paneId: number
  data: string
}

export interface TerminalResizeOptions {
  paneId: number
  rows: number
  cols: number
}

// ===== Shell相关类型（从shell模块重新导出） =====

export type { ShellInfo } from '../shell/types'

export interface CreateTerminalWithShellOptions {
  shellName?: string
  rows: number
  cols: number
}

// ===== 事件相关类型 =====

export interface TerminalOutputEvent {
  paneId: number
  data: string
}

export interface TerminalExitEvent {
  paneId: number
  exitCode: number | null
}

export interface TerminalResizeEvent {
  paneId: number
  rows: number
  cols: number
}

// ===== 终端统计信息类型 =====

export interface TerminalStats {
  total: number
  active: number
  ids: number[]
}

// ===== 终端操作结果类型 =====

export interface TerminalOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

// ===== 终端重试选项类型 =====

export interface TerminalRetryOptions {
  retries?: number
  retryDelay?: number
}

// ===== 批量终端操作类型 =====

export interface BatchTerminalResize {
  paneId: number
  rows: number
  cols: number
}

// ===== 终端配置类型 =====

export interface TerminalConfig {
  scrollback: number
  shell: ShellConfig
  cursor: CursorConfig
  behavior: TerminalBehaviorConfig
}

export interface ShellConfig {
  default: string
  args: string[]
  workingDirectory: string
}

export interface CursorConfig {
  style: 'block' | 'underline' | 'beam'
  blink: boolean
  color: string
  thickness: number
}

export interface TerminalBehaviorConfig {
  closeOnExit: boolean
  confirmOnExit: boolean
  scrollOnOutput: boolean
  copyOnSelect: boolean
}

export interface TerminalConfigValidationResult {
  valid: boolean
  errors?: string[]
  warnings?: string[]
}

export interface SystemShellsResult {
  shells: DetectedShell[]
  currentDefault?: string
}

export interface DetectedShell {
  name: string
  path: string
  version?: string
  description?: string
  available: boolean
}

// ===== 通用响应类型 =====

export interface APIResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
  code?: string
}
