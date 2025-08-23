/**
 * 终端业务领域类型定义
 */

import type { BaseConfig } from '../core'

// ===== 基础终端类型 =====

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

export interface CreateTerminalWithShellOptions {
  shellName?: string
  rows: number
  cols: number
}

// ===== 事件类型 =====

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

// ===== 统计信息类型 =====

export interface TerminalStats {
  total: number
  active: number
  ids: number[]
}

// ===== 操作结果类型 =====

export interface TerminalOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

// ===== 批量操作类型 =====

export interface BatchTerminalResize {
  paneId: number
  rows: number
  cols: number
}

// ===== 主题配置类型 =====

export interface TerminalTheme {
  background: string
  foreground: string
}

// ===== Shell配置类型 =====

export interface ShellConfig {
  default: string
  args: string[]
  workingDirectory: string
}

export interface DetectedShell {
  name: string
  path: string
  version?: string
  description?: string
  available: boolean
}

export interface SystemShellsResult {
  shells: DetectedShell[]
  currentDefault?: string
}

export interface ShellInfo {
  name: string
  path: string
  version?: string
  isDefault?: boolean
}

// ===== 光标配置类型 =====

export interface CursorConfig {
  style: 'block' | 'underline' | 'beam'
  blink: boolean
  color: string
  thickness: number
}

// ===== 行为配置类型 =====

export interface TerminalBehaviorConfig {
  closeOnExit: boolean
  confirmOnExit: boolean
  scrollOnOutput: boolean
  copyOnSelect: boolean
}

// ===== 完整终端配置类型 =====

export interface TerminalConfig extends BaseConfig {
  fontFamily: string
  fontSize: number
  cursorBlink: boolean
  theme: TerminalTheme
  scrollback: number
  shell: ShellConfig
  cursor: CursorConfig
  behavior: TerminalBehaviorConfig

  // 高级配置
  allowTransparency?: boolean
  allowProposedApi?: boolean
  altClickMovesCursor?: boolean
  convertEol?: boolean
  cursorStyle?: 'block' | 'underline' | 'bar'
  cursorWidth?: number
  disableStdin?: boolean
  drawBoldTextInBrightColors?: boolean
  fastScrollModifier?: 'alt' | 'ctrl' | 'shift'
  fastScrollSensitivity?: number
  fontWeight?: number
  fontWeightBold?: number
  letterSpacing?: number
  lineHeight?: number
  linkTooltipHoverDuration?: number
  logLevel?: 'debug' | 'info' | 'warn' | 'error' | 'off'
  macOptionIsMeta?: boolean
  macOptionClickForcesSelection?: boolean
  minimumContrastRatio?: number
  rightClickSelectsWord?: boolean
  screenReaderMode?: boolean
  scrollSensitivity?: number
  smoothScrollDuration?: number
  tabStopWidth?: number
  windowsMode?: boolean
  wordSeparator?: string
}

// ===== 配置验证类型 =====

export interface TerminalConfigValidationResult {
  valid: boolean
  errors?: string[]
  warnings?: string[]
}

// ===== 重试选项类型 =====

export interface TerminalRetryOptions {
  retries?: number
  retryDelay?: number
}

// ===== 事件常量类型 =====

export interface TerminalEvents {
  OUTPUT: 'terminal_output'
  EXIT: 'terminal_exit'
}
