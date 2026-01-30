/**
 * 存储API类型定义
 */
export type {
  SessionState,
  TabState,
  AgentTerminalTabState,
  TerminalTabState,
  SettingsTabState,
  RuntimeTerminalState,
  DataQuery,
  SaveOptions,
} from '@/types'

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
