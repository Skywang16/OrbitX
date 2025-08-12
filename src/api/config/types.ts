/**
 * 配置模块相关的类型定义
 */

// 基础配置类型
export interface AIConfig {
  enabled: boolean
  apiKey?: string
  model?: string
}

export interface AppConfig {
  theme: string
  language: string
  ai: AIConfig
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

// 配置API错误类
export class ConfigApiError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}

// ===== 配置操作结果类型 =====

export interface ConfigOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

// ===== 配置部分更新类型 =====

export interface ConfigSectionUpdate<T = any> {
  section: string
  updates: Partial<T>
}

// ===== 主题相关类型 =====

// 重新导出主题相关类型
export type { ThemeConfigStatus, ThemeInfo } from '@/types/theme'
