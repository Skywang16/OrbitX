export interface BaseAPIResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
  code?: string
}

export interface APIErrorInfo {
  message: string
  code?: string
  details?: Record<string, unknown>
}

export interface NetworkInfo {
  interfaces: Array<{
    name: string
    ip: string
    mac: string
  }>
}

export interface RequestConfig {
  timeout?: number
  retries?: number
  headers?: Record<string, string>
}

export interface ResponseMetadata {
  requestId?: string
  timestamp: string
  duration?: number
  cached?: boolean
}
