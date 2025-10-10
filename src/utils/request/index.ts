import { invoke as tauriInvoke } from '@tauri-apps/api/core'

/**
 * API调用选项
 */
export interface APIOptions {
  signal?: AbortSignal
  timeout?: number
}

/**
 * API错误类型
 */
export class APIError extends Error {
  constructor(
    message: string,
    public code: string = 'UNKNOWN',
    public originalError?: unknown
  ) {
    super(message)
    this.name = 'APIError'
  }
}

/**
 * 简单的API调用封装
 * 提供错误处理、超时控制、日志记录
 */
export class APIClient {
  private static instance: APIClient

  private constructor() {}

  static getInstance(): APIClient {
    if (!APIClient.instance) {
      APIClient.instance = new APIClient()
    }
    return APIClient.instance
  }

  /**
   * 执行Tauri命令调用
   */
  async invoke<T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> {
    try {
      // 检查是否已被取消
      if (options?.signal?.aborted) {
        throw new APIError('Request was aborted', 'ABORTED')
      }

      // 执行实际的API调用
      const result = await this.executeCommand<T>(command, args, options)

      return result
    } catch (error) {
      const apiError =
        error instanceof APIError
          ? error
          : new APIError(`Command '${command}' failed: ${error}`, 'COMMAND_FAILED', error)

      throw apiError
    }
  }

  /**
   * 执行实际的Tauri命令
   */
  private async executeCommand<T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> {
    const timeout = options?.timeout || 300000

    return new Promise((resolve, reject) => {
      const abortHandler = () => {
        reject(new APIError('Request was aborted', 'ABORTED'))
      }

      const timeoutId: NodeJS.Timeout = setTimeout(() => {
        reject(new APIError(`Request timeout after ${timeout}ms`, 'TIMEOUT'))
      }, timeout)

      // 如果提供了signal，监听abort事件
      if (options?.signal) {
        options.signal.addEventListener('abort', abortHandler)
      }

      // 执行实际的API调用
      tauriInvoke<T>(command, args)
        .then(result => {
          // 清理资源
          this.cleanup(options?.signal, abortHandler, timeoutId)
          resolve(result)
        })
        .catch(error => {
          // 清理资源
          this.cleanup(options?.signal, abortHandler, timeoutId)
          reject(error)
        })
    })
  }

  /**
   * 清理资源
   */
  private cleanup(signal?: AbortSignal, abortHandler?: () => void, timeoutId?: NodeJS.Timeout) {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }
    if (signal && abortHandler) {
      signal.removeEventListener('abort', abortHandler)
    }
  }

  /**
   * 批量调用API
   */
  async batchInvoke<T>(
    commands: Array<{ command: string; args?: Record<string, unknown> }>,
    options?: APIOptions
  ): Promise<T[]> {
    const promises = commands.map(({ command, args }) => this.invoke<T>(command, args, options))
    return Promise.all(promises)
  }

  /**
   * 带重试的API调用
   */
  async invokeWithRetry<T>(
    command: string,
    args?: Record<string, unknown>,
    options?: APIOptions & { retries?: number; retryDelay?: number }
  ): Promise<T> {
    const maxRetries = options?.retries || 3
    const retryDelay = options?.retryDelay || 1000

    let lastError: Error

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await this.invoke<T>(command, args, options)
      } catch (error) {
        lastError = error as Error

        if (attempt < maxRetries) {
          await new Promise(resolve => setTimeout(resolve, retryDelay))
        }
      }
    }

    throw lastError!
  }
}

/**
 * 全局API实例
 */
export const api = APIClient.getInstance()

/**
 * 便捷的API调用函数
 */
export const apiClient = api

/**
 * 后端统一API响应结构
 */
export interface ApiResponse<T> {
  code: number
  message?: string
  data?: T
}

import { createMessage } from '@/ui'

/**
 * 统一的API调用函数 - 处理新的后端响应格式
 */
export const invoke = async <T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> => {
  const response = await api.invoke<ApiResponse<T>>(command, args, options)

  if (response.code === 200) {
    // 如果后端返回了成功消息，显示它
    if (response.message) {
      createMessage.success(response.message)
    }
    return response.data as T
  } else {
    // 统一错误提示 - 后端已完成国际化
    createMessage.error(response.message || '操作失败')
    throw new APIError(response.message || '操作失败', String(response.code))
  }
}
