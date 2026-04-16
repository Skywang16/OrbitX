import { invoke as tauriInvoke } from '@tauri-apps/api/core'

/**
 * API call options
 */
export interface APIOptions {
  signal?: AbortSignal
  timeout?: number
  /** Silent mode, don't show error messages */
  silent?: boolean
}

/**
 * API error type
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
 * Simple API call wrapper
 * Provides error handling, timeout control, logging
 */
export class APIClient {
  private static instance: APIClient

  private constructor() {}

  static getInstance = (): APIClient => {
    if (!APIClient.instance) {
      APIClient.instance = new APIClient()
    }
    return APIClient.instance
  }

  /**
   * Execute Tauri command call
   */
  invoke = async <T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> => {
    try {
      // Check if already aborted
      if (options?.signal?.aborted) {
        throw new APIError('Request was aborted', 'ABORTED')
      }

      // Execute actual API call
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
   * Execute actual Tauri command
   */
  private executeCommand = async <T>(
    command: string,
    args?: Record<string, unknown>,
    options?: APIOptions
  ): Promise<T> => {
    const timeout = options?.timeout || 300000

    return new Promise((resolve, reject) => {
      let timeoutId: NodeJS.Timeout | null = null
      const abortHandler = () => {
        this.cleanup(options?.signal, abortHandler, timeoutId || undefined)
        reject(new APIError('Request was aborted', 'ABORTED'))
      }

      timeoutId = setTimeout(() => {
        this.cleanup(options?.signal, abortHandler, timeoutId || undefined)
        reject(new APIError(`Request timeout after ${timeout}ms`, 'TIMEOUT'))
      }, timeout)

      // If signal provided, listen for abort event
      if (options?.signal) {
        options.signal.addEventListener('abort', abortHandler)
      }

      // Execute actual API call
      tauriInvoke<T>(command, args)
        .then(result => {
          // Cleanup resources
          this.cleanup(options?.signal, abortHandler, timeoutId || undefined)
          resolve(result)
        })
        .catch(error => {
          // Cleanup resources
          this.cleanup(options?.signal, abortHandler, timeoutId || undefined)
          reject(error)
        })
    })
  }

  /**
   * Cleanup resources
   */
  private cleanup = (signal?: AbortSignal, abortHandler?: () => void, timeoutId?: NodeJS.Timeout) => {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }
    if (signal && abortHandler) {
      signal.removeEventListener('abort', abortHandler)
    }
  }

  /**
   * Batch API calls
   */
  batchInvoke = async <T>(
    commands: Array<{ command: string; args?: Record<string, unknown> }>,
    options?: APIOptions
  ): Promise<T[]> => {
    const promises = commands.map(({ command, args }) => this.invoke<T>(command, args, options))
    return Promise.all(promises)
  }

  /**
   * API call with retry
   */
  invokeWithRetry = async <T>(
    command: string,
    args?: Record<string, unknown>,
    options?: APIOptions & { retries?: number; retryDelay?: number }
  ): Promise<T> => {
    const maxRetries = options?.retries || 3
    const retryDelay = options?.retryDelay || 1000

    let lastError: Error

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await this.invoke<T>(command, args, options)
      } catch (error) {
        lastError = error as Error
        console.warn(`[API Retry] command=${command} attempt=${attempt + 1}/${maxRetries + 1} error=`, error)

        if (attempt < maxRetries) {
          await new Promise(resolve => setTimeout(resolve, retryDelay))
        }
      }
    }

    throw lastError!
  }
}

/**
 * Global API instance
 */
export const api = APIClient.getInstance()

/**
 * Convenient API call function
 */
export const apiClient = api

/**
 * Backend unified API response structure
 */
export interface ApiResponse<T> {
  code: number
  message?: string
  data?: T
}

import { createMessage } from '@/ui'

/**
 * Unified API call function - handles new backend response format
 */
export const invoke = async <T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> => {
  const response = await api.invoke<ApiResponse<T>>(command, args, options)

  if (response.code === 200) {
    // If backend returned success message, show it (not in silent mode)
    if (response.message && !options?.silent) {
      createMessage.success(response.message)
    }
    return response.data as T
  } else {
    // Unified error prompt - backend has i18n (not in silent mode)
    console.error(`[API Error] command: ${command}, code: ${response.code}, message: ${response.message}`, args)
    if (!options?.silent) {
      createMessage.error(response.message || 'Operation failed')
    }
    throw new APIError(response.message || 'Operation failed', String(response.code))
  }
}
