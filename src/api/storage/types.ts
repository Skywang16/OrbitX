/**
 * 存储API类型定义
 */
import { ConfigSection } from '@/types'
import type { AppConfig } from '@/api/config/types'

export type { SessionState, DataQuery, SaveOptions, ConfigSection } from '@/types'

/**
 * 存储操作结果
 */
export interface StorageOperationResult {
  success: boolean
  error?: string
}

/**
 * 存储API选项
 */
export interface StorageAPIOptions {
  timeout?: number
  retries?: number
}

export type AppSection = AppConfig['app']
export type AppearanceSection = AppConfig['appearance']
export type TerminalSection = AppConfig['terminal']
export type ShortcutsSection = AppConfig['shortcuts']
export type AiSection = Record<string, never>

export interface ConfigSectionMap {
  [ConfigSection.App]: AppSection
  [ConfigSection.Appearance]: AppearanceSection
  [ConfigSection.Terminal]: TerminalSection
  [ConfigSection.Shortcuts]: ShortcutsSection
  [ConfigSection.Ai]: AiSection
}
