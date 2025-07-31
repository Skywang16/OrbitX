/**
 * 补全模块相关的类型定义
 */

// ===== 补全相关类型 =====

export interface CompletionItem {
  text: string
  displayText?: string
  description?: string
  kind: string
  score: number
  source: string
}

export interface CompletionRequest {
  input: string
  cursorPosition: number
  workingDirectory: string
  maxResults?: number
}

export interface CompletionResponse {
  items: CompletionItem[]
  replaceStart: number
  replaceEnd: number
  hasMore: boolean
}

// ===== 增强补全相关类型 =====

export interface EnhancedCompletionItem {
  text: string
  displayText?: string
  description: string
  icon: string
  category: string
  priority: number
  metadata: Record<string, string>
}

export interface EnhancedCompletionPosition {
  x: number
  y: number
}

export interface EnhancedCompletionResponse {
  completions: EnhancedCompletionItem[]
  position: EnhancedCompletionPosition
  hasShellCompletions: boolean
}

// ===== 统计信息类型 =====

export interface CompletionStats {
  providerCount: number
  cacheStats?: {
    totalEntries: number
    capacity: number
    expiredEntries: number
    hitRate: number
  }
}

// ===== 补全引擎状态类型 =====

export interface CompletionEngineStatus {
  initialized: boolean
  ready: boolean
}

// ===== 补全操作结果类型 =====

export interface CompletionResult<T = CompletionResponse> {
  success: boolean
  data?: T
  error?: string
}

// ===== 重试选项类型 =====

export interface CompletionRetryOptions {
  retries?: number
  retryDelay?: number
}
