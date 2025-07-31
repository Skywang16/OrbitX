/**
 * 补全管理相关的API接口
 */

import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type {
  CompletionEngineStatus,
  CompletionRequest,
  CompletionResponse,
  CompletionResult,
  CompletionRetryOptions,
  CompletionStats,
} from './types'

/**
 * 补全管理API
 * 提供智能补全相关的功能
 */
export class CompletionAPI {
  /**
   * 初始化补全引擎
   */
  async initEngine(): Promise<void> {
    try {
      return await invoke<void>('init_completion_engine')
    } catch (error) {
      throw new Error(handleError(error, '初始化补全引擎失败'))
    }
  }

  /**
   * 获取补全建议
   */
  async getCompletions(request: CompletionRequest): Promise<CompletionResponse> {
    try {
      return await invoke<CompletionResponse>('get_completions', {
        input: request.input,
        cursorPosition: request.cursorPosition,
        workingDirectory: request.workingDirectory,
        maxResults: request.maxResults,
      })
    } catch (error) {
      throw new Error(handleError(error, '获取补全建议失败'))
    }
  }

  /**
   * 获取增强补全建议
   */
  async getEnhancedCompletions(currentLine: string, cursorPosition: number, workingDirectory: string): Promise<any> {
    try {
      return await invoke('get_enhanced_completions', {
        currentLine,
        cursorPosition,
        workingDirectory,
      })
    } catch (error) {
      throw new Error(handleError(error, '获取增强补全建议失败'))
    }
  }

  /**
   * 清理缓存
   */
  async clearCache(): Promise<void> {
    try {
      return await invoke<void>('clear_completion_cache')
    } catch (error) {
      throw new Error(handleError(error, '清理补全缓存失败'))
    }
  }

  /**
   * 获取统计信息
   */
  async getStats(): Promise<CompletionStats> {
    try {
      const stats = await invoke<string>('get_completion_stats')
      return JSON.parse(stats) as CompletionStats
    } catch (error) {
      throw new Error(handleError(error, '获取补全统计信息失败'))
    }
  }

  /**
   * 带重试的补全获取
   */
  async getCompletionsWithRetry(
    request: CompletionRequest,
    retryOptions?: CompletionRetryOptions
  ): Promise<CompletionResponse> {
    const maxRetries = retryOptions?.retries || 3
    const retryDelay = retryOptions?.retryDelay || 1000

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await this.getCompletions(request)
      } catch (error) {
        if (attempt === maxRetries) {
          throw new Error(handleError(error, '获取补全建议失败（重试后）'))
        }
        await new Promise(resolve => setTimeout(resolve, retryDelay))
      }
    }
    throw new Error('获取补全建议失败（重试后）')
  }

  /**
   * 批量获取多个补全请求
   */
  async getBatchCompletions(requests: CompletionRequest[]): Promise<CompletionResponse[]> {
    try {
      const results: CompletionResponse[] = []
      for (const request of requests) {
        const response = await this.getCompletions(request)
        results.push(response)
      }
      return results
    } catch (error) {
      throw new Error(handleError(error, '批量获取补全建议失败'))
    }
  }

  /**
   * 安全的补全获取（带错误处理）
   */
  async safeGetCompletions(request: CompletionRequest): Promise<CompletionResult> {
    try {
      const data = await this.getCompletions(request)
      return { success: true, data }
    } catch (error) {
      return {
        success: false,
        error: handleError(error, '获取补全建议失败'),
      }
    }
  }

  /**
   * 检查补全引擎状态
   */
  async getEngineStatus(): Promise<CompletionEngineStatus> {
    try {
      // 尝试获取统计信息来检查引擎状态
      await this.getStats()
      return { initialized: true, ready: true }
    } catch (error) {
      handleError(error, '检查补全引擎状态失败')
      return { initialized: false, ready: false }
    }
  }
}

/**
 * 补全API实例
 */
export const completionAPI = new CompletionAPI()

/**
 * 便捷的补全操作函数
 */
export const completion = {
  // 基本操作
  init: () => completionAPI.initEngine(),
  get: (request: CompletionRequest) => completionAPI.getCompletions(request),
  clearCache: () => completionAPI.clearCache(),
  getStats: () => completionAPI.getStats(),

  // 高级功能
  getWithRetry: (request: CompletionRequest, retryOptions?: CompletionRetryOptions) =>
    completionAPI.getCompletionsWithRetry(request, retryOptions),
  getBatch: (requests: CompletionRequest[]) => completionAPI.getBatchCompletions(requests),
  safeGet: (request: CompletionRequest) => completionAPI.safeGetCompletions(request),
  getStatus: () => completionAPI.getEngineStatus(),
}

// 重新导出类型
export type * from './types'
