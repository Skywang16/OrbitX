/**
 * AI 模块类型定义
 */

// 重新导出主要类型
export type {
  AIHealthStatus,
  AIModelConfig,
  AISettings,
  AIStats,
  Conversation,
  Message,
} from '@/types'

// ===== 会话管理类型 =====

/** 原始会话数据格式（后端返回） */
export interface RawConversation {
  id: number
  title: string
  messageCount: number
  lastMessagePreview?: string
  createdAt: string
  updatedAt: string
}

/** 原始消息数据格式（后端返回） */
export interface RawMessage {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  content: string
  stepsJson?: string | null
  status?: 'pending' | 'streaming' | 'complete' | 'error'
  durationMs?: number | null
  createdAt: string
}

// ===== 工具调用类型 =====

/** 代码符号信息 */
export interface CodeSymbol {
  /** 符号名称 */
  name: string
  /** 符号类型 */
  type: string
  /** 所在行号 */
  line: number
  /** 所在列号 */
  column: number
  /** 符号范围 */
  range?: {
    start: { line: number; column: number }
    end: { line: number; column: number }
  }
}

/** 代码分析结果 */
export interface CodeAnalysis {
  /** 文件路径 */
  file: string
  /** 编程语言 */
  language: string
  /** 符号列表 */
  symbols: CodeSymbol[]
  /** 导入语句 */
  imports: string[]
  /** 导出语句 */
  exports: string[]
  /** 代码复杂度 */
  complexity?: number
  /** 代码行数 */
  lines?: number
}

/** 分析代码参数 */
export interface AnalyzeCodeParams {
  /** 文件路径或代码内容 */
  source: string
  /** 是否为文件路径 */
  isFilePath?: boolean
  /** 编程语言（可选，自动检测） */
  language?: string
  /** 分析选项 */
  options?: {
    /** 是否包含符号信息 */
    includeSymbols?: boolean
    /** 是否包含导入导出 */
    includeImports?: boolean
    /** 是否计算复杂度 */
    includeComplexity?: boolean
  }
}

/** 分析结果类型 */
export type AnalysisResult = CodeAnalysis

/** Web 请求类型 */
export interface WebFetchRequest {
  /** 请求URL */
  url: string
  /** 请求方法 */
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  /** 请求头 */
  headers?: Record<string, string>
  /** 请求体 */
  body?: string
  /** 超时时间（毫秒） */
  timeout?: number
}

/** Web 响应类型 */
export interface WebFetchResponse {
  /** 响应状态码 */
  status: number
  /** 响应头 */
  headers: Record<string, string>
  /** 响应体 */
  body: string
  /** 响应URL */
  url: string
  /** 是否成功 */
  ok: boolean
}