import type { BaseConfig } from '../core'

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

export interface TerminalExitEvent {
  paneId: number
  exitCode: number | null
}

export interface TerminalResizeEvent {
  paneId: number
  rows: number
  cols: number
}

export interface TerminalStats {
  total: number
  active: number
  ids: number[]
}

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

export interface TerminalTheme {
  background: string
  foreground: string
}

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

  // Advanced terminal configuration options
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
