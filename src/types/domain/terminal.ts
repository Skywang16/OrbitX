import type { BaseConfig } from '../core'

// ===== Terminal basic operation types =====

export interface TerminalCreateOptions {
  rows: number
  cols: number
  cwd?: string
  threadId?: number
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
  threadId?: number
}

// ===== Replay types =====

export interface ReplayEvent {
  data: string
  cols: number
  rows: number
}

export interface ProcessReplayEvent {
  events: ReplayEvent[]
}

// ===== Event types =====

export interface TerminalExitEvent {
  paneId: number
  exitCode: number | null
}

export interface TerminalResizeEvent {
  paneId: number
  rows: number
  cols: number
}

export type CommandStatus = { type: 'ready' } | { type: 'running' } | { type: 'finished'; exitCode: number | null }

export interface CommandEventPayload {
  id: number
  exit_code: number | null
  status: CommandStatus
  command_line: string | null
  working_directory: string | null
}

// ===== Stats =====

export interface TerminalStats {
  total: number
  active: number
  ids: number[]
}

// ===== Operation result =====

export interface TerminalOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

export interface BatchTerminalResize {
  paneId: number
  rows: number
  cols: number
}

// ===== Theme =====

export interface TerminalTheme {
  background: string
  foreground: string
}

// ===== Shell info (canonical definition, mirrors backend ShellInfo) =====

export interface ShellInfo {
  name: string
  path: string
  displayName: string
  args?: string[]
}

// ===== Terminal configuration =====

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

export interface TerminalConfig extends BaseConfig {
  fontFamily: string
  fontSize: number
  cursorBlink: boolean
  theme: TerminalTheme
  scrollback: number
  shell: ShellConfig
  cursor: CursorConfig
  behavior: TerminalBehaviorConfig

  // Advanced xterm.js options
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

export interface TerminalConfigValidationResult {
  valid: boolean
  errors?: string[]
  warnings?: string[]
}

export interface TerminalRetryOptions {
  retries?: number
  retryDelay?: number
}

export interface TerminalEvents {
  EXIT: 'terminal_exit'
}
