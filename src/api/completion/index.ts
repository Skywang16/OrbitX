/**
 * 补全管理 API
 * 
 * 提供智能补全的统一接口，包括：
 * - 补全引擎管理
 * - 补全建议获取
 * - 统计和状态监控
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type {
  CompletionEngineStatus,
  CompletionRequest,
  CompletionResponse,
  CompletionStats,
} from './types'

/**
 * 补全 API 接口类
 */
export class CompletionApi {
  async initEngine(): Promise<void> {
    try {
      await invoke('init_completion_engine')
    } catch (error) {
      throw new Error(handleError(error, '初始化补全引擎失败'))
    }
  }

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

  async getEnhancedCompletions(
    currentLine: string, 
    cursorPosition: number, 
    workingDirectory: string
  ): Promise<any> {
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

  async clearCache(): Promise<void> {
    try {
      await invoke('clear_completion_cache')
    } catch (error) {
      throw new Error(handleError(error, '清理补全缓存失败'))
    }
  }

  async getStats(): Promise<CompletionStats> {
    try {
      const stats = await invoke<string>('get_completion_stats')
      return JSON.parse(stats) as CompletionStats
    } catch (error) {
      throw new Error(handleError(error, '获取补全统计信息失败'))
    }
  }

  async getEngineStatus(): Promise<CompletionEngineStatus> {
    try {
      await this.getStats()
      return { initialized: true, ready: true }
    } catch (error) {
      console.warn(handleError(error, '检查补全引擎状态失败'))
      return { initialized: false, ready: false }
    }
  }
}

// 导出单例实例
export const completionApi = new CompletionApi()

// 导出类型
export type * from './types'

// 默认导出
export default completionApi