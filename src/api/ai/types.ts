/**
 * AI 模块类型定义
 */

// 重新导出主要类型
export type { AIHealthStatus, AIModelConfig, AISettings, AIStats, Conversation, Message } from '@/types'

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

/** 单个文件的代码分析结果 */
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
}

/** 批量代码分析结果 */
export interface BatchCodeAnalysis {
  /** 分析结果列表 */
  analyses: CodeAnalysis[]
  /** 总文件数 */
  total_files: number
  /** 成功分析的文件数 */
  success_count: number
  /** 失败的文件数 */
  error_count: number
}

/** 分析代码参数 */
export interface AnalyzeCodeParams {
  /** 文件路径 */
  path: string
  /** 是否递归分析 */
  recursive?: boolean
  /** 包含的文件模式 */
  include?: string[]
  /** 排除的文件模式 */
  exclude?: string[]
}

/** 分析结果类型 */
export type AnalysisResult = BatchCodeAnalysis

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
  /** 响应状态文本 */
  status_text: string
  /** 响应头 */
  headers: Record<string, string>
  /** 响应数据 */
  data: string
  /** 最终URL */
  final_url: string
  /** 是否成功 */
  success: boolean
  /** 错误信息 */
  error?: string
  /** 响应时间 */
  response_time: number
  /** 内容类型 */
  content_type?: string
  /** 内容长度 */
  content_length?: number
}
