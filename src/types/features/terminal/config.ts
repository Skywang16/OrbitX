/**
 * 终端配置相关类型定义
 */

// ===== 终端主题类型 =====

export interface TerminalTheme {
  background: string
  foreground: string
}

// ===== 终端配置类型 =====

export interface TerminalConfig {
  fontFamily: string
  fontSize: number
  cursorBlink: boolean
  theme: TerminalTheme
  // 添加更多编码和渲染相关配置
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
  scrollback?: number
  scrollSensitivity?: number
  smoothScrollDuration?: number
  tabStopWidth?: number
  windowsMode?: boolean
  wordSeparator?: string
}

// ===== 终端事件常量类型 =====

export interface TerminalEvents {
  OUTPUT: 'terminal_output'
  EXIT: 'terminal_exit'
}
