/**
 * UI设置相关类型定义
 */

// ===== 应用设置类型 =====

export interface AppSettings {
  theme: {
    mode: any
    terminalTheme: string
  }
  terminal: {
    fontFamily: string
    fontSize: number
    cursorStyle: string
    cursorBlink: boolean
    scrollback: number
  }
  window: {
    opacity: number
    alwaysOnTop: boolean
    startMaximized: boolean
  }
  general: {
    language: string
    autoSave: boolean
    confirmOnExit: boolean
  }
}
