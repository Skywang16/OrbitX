/**
 * 存储API类型定义
 */

// 重新导出存储类型
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
