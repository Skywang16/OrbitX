/**
 * 补全管理 API
 *
 * 提供智能补全的统一接口，包括：
 * - 补全引擎管理
 * - 补全建议获取
 * - 统计和状态监控
 */

import { invoke } from '@/utils/request'
import type { CompletionRequest, CompletionResponse, CompletionStats } from './types'

/**
 * 补全 API 接口类
 */
export class CompletionApi {
  initEngine = async (): Promise<void> => {
    await invoke<void>('completion_init_engine')
  }

  getCompletions = async (request: CompletionRequest): Promise<CompletionResponse> => {
    return await invoke<CompletionResponse>('completion_get', {
      input: request.input,
      cursorPosition: request.cursorPosition,
      workingDirectory: request.workingDirectory,
      maxResults: request.maxResults,
    })
  }

  clearCache = async (): Promise<void> => {
    await invoke<void>('completion_clear_cache')
  }

  getStats = async (): Promise<CompletionStats> => {
    return await invoke<CompletionStats>('completion_get_stats')
  }
}

export const completionApi = new CompletionApi()
export type * from './types'
export default completionApi
