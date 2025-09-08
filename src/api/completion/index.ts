/**
 * 补全管理 API
 *
 * 提供智能补全的统一接口，包括：
 * - 补全引擎管理
 * - 补全建议获取
 * - 统计和状态监控
 */

import { invoke } from '@/utils/request'
import type { CompletionEngineStatus, CompletionRequest, CompletionResponse, CompletionStats } from './types'

/**
 * 补全 API 接口类
 */
export class CompletionApi {
  async initEngine(): Promise<void> {
    await invoke<void>('init_completion_engine')
  }

  async getCompletions(request: CompletionRequest): Promise<CompletionResponse> {
    return await invoke<CompletionResponse>('get_completions', {
      input: request.input,
      cursorPosition: request.cursorPosition,
      workingDirectory: request.workingDirectory,
      maxResults: request.maxResults,
    })
  }

  async clearCache(): Promise<void> {
    await invoke<void>('clear_completion_cache')
  }

  async getStats(): Promise<CompletionStats> {
    const stats = await invoke<string>('get_completion_stats')
    return JSON.parse(stats) as CompletionStats
  }

  async getEngineStatus(): Promise<CompletionEngineStatus> {
    await this.getStats()
    return { initialized: true, ready: true }
  }
}

// 导出单例实例
export const completionApi = new CompletionApi()

// 导出类型
export type * from './types'

// 默认导出
export default completionApi
