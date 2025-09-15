/**
 * 配置模块相关的类型定义
 */

import type { ShortcutsConfig } from '@/types/domain/shortcuts'

// 配置 API 错误类
export class ConfigApiError extends Error {
  constructor(
    message: string,
    public cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}

// 基础配置类型
export interface AIConfig {
  enabled: boolean
  apiKey?: string
  model?: string
}

export interface AppConfig {
  version: string
  app: {
    language: string
    confirm_on_exit: boolean
    startup_behavior: string
  }
  appearance: {
    ui_scale: number
    animations_enabled: boolean
    opacity: number
    theme_config: {
      auto_switch_time: string
      terminal_theme: string
      light_theme: string
      dark_theme: string
      follow_system: boolean
    }
    font: {
      family: string
      size: number
      weight: string
      style: string
      lineHeight: number
      letterSpacing: number
    }
  }
  terminal: {
    scrollback: number
    shell: {
      default: string
      args: string[]
      working_directory: string
    }
    cursor: {
      style: string
      blink: boolean
      color: string
      thickness: number
    }
    behavior: {
      close_on_exit: boolean
      confirm_close: boolean
    }
  }
  shortcuts: {
    global: ShortcutsConfig
    terminal: ShortcutsConfig
    custom: ShortcutsConfig
  }
}

export interface ConfigFileInfo {
  path: string
  exists: boolean
  lastModified?: number
}

export interface TerminalConfig {
  shell: string
  fontSize: number
  fontFamily: string
}

export interface CursorConfig {
  style: string
  blinking: boolean
}

// ===== 配置部分更新类型 =====

export interface ConfigSectionUpdate<T = unknown> {
  section: string
  updates: Partial<T>
}

// ===== 主题相关类型 =====

// 重新导出主题相关类型
export type { ThemeConfigStatus, ThemeInfo, Theme } from '@/types'
