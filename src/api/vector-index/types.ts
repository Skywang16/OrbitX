/**
 * 向量索引模块相关的类型定义
 */

// ===== 配置与状态 =====

export interface VectorIndexConfig {
  qdrantUrl: string
  qdrantApiKey: string | null
  collectionName: string
  embeddingModelId: string // 必需：关联的embedding模型ID
  maxConcurrentFiles: number // 性能调优参数
}

export interface VectorIndexStatus {
  isInitialized: boolean
  totalVectors: number
  lastUpdated: string | null
  collectionName?: string
}

// ===== 构建与统计 =====

export interface IndexStats {
  totalFiles: number
  totalChunks: number
  processingTime: number // milliseconds
  // 扩展字段（后端存在但前端可选使用）
  uploadedVectors?: number
}

// ===== 搜索 =====

export interface VectorSearchOptions {
  query: string
  maxResults?: number
  minScore?: number
  directoryFilter?: string
  languageFilter?: string
}

export interface VectorSearchResult {
  filePath: string
  content: string
  startLine: number
  endLine: number
  language: string
  chunkType: string
  score: number
}

// ===== 事件 =====

export type VectorIndexEventType = 'progress' | 'completed' | 'error' | 'status_changed'

export interface VectorIndexEvent<T = unknown> {
  type: VectorIndexEventType
  data: T
}

export interface BuildProgressPayload {
  progress: number // 0-1
  processedFiles: number
  totalFiles: number
  currentFile?: string
}

export interface BuildCompletedPayload {
  totalFiles: number
  totalChunks: number
  elapsedTime: number // ms
}

export interface ErrorPayload {
  message: string
}
