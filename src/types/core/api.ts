/**
 * API相关的核心类型定义
 * 不依赖业务类型，提供API通用结构
 */

// ===== 通用响应类型 =====

export interface BaseAPIResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
  code?: string
}

// ===== 通用错误类型 =====

export interface APIErrorInfo {
  message: string
  code?: string
  details?: Record<string, unknown>
}

// ===== 网络相关类型 =====

export interface NetworkInfo {
  interfaces: Array<{
    name: string
    ip: string
    mac: string
  }>
}

// ===== 请求配置类型 =====

export interface RequestConfig {
  timeout?: number
  retries?: number
  headers?: Record<string, string>
}

// ===== 响应元数据类型 =====

export interface ResponseMetadata {
  requestId?: string
  timestamp: string
  duration?: number
  cached?: boolean
}
