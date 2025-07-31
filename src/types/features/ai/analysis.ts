/**
 * AI分析相关类型定义
 * 包括命令解释和错误分析
 */

// ===== 命令解释类型 =====

export interface CommandExplanation {
  command: string
  explanation: string
  breakdown?: Array<{
    part: string
    description: string
  }>
  risks?: Array<{
    level: 'low' | 'medium' | 'high'
    description: string
  }>
  alternatives?: Array<{
    command: string
    description: string
    reason: string
  }>
}

// ===== 错误分析类型 =====

export interface ErrorAnalysis {
  error: string
  command: string
  analysis: string
  possibleCauses: string[]
  solutions: Array<{
    description: string
    command?: string
    priority: 'high' | 'medium' | 'low'
  }>
  relatedDocs?: Array<{
    title: string
    url: string
  }>
}

// ===== 上下文管理类型 =====

export interface TerminalContext {
  workingDirectory: string
  environment: Record<string, string>
  commandHistory: string[]
  currentCommand?: string
  lastOutput?: string
  systemInfo?: {
    platform: string
    shell: string
    user: string
  }
}

// ===== 缓存相关类型 =====

export interface CacheEntry<T = unknown> {
  key: string
  value: T
  timestamp: Date
  ttl: number
  hits: number
}

export interface CacheStats {
  totalEntries: number
  hitRate: number
  memoryUsage: number
  oldestEntry?: Date
  newestEntry?: Date
}
