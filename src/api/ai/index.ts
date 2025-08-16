/**
 * AI 模块 API
 *
 * 提供 AI 相关功能的统一接口，包括：
 * - 模型管理
 * - 会话管理
 * - 工具调用
 * - 设置管理
 */

import type { AIHealthStatus, AIModelConfig, AISettings, AIStats } from '@/types'
import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'

// 导入子模块
import { conversationAPI } from './conversations'
import type { AnalyzeCodeParams, AnalysisResult, WebFetchRequest, WebFetchResponse } from './tool'
import { analyzeCode, webFetchHeadless } from './tool'

/**
 * AI API 接口类
 */
export class AiApi {
  // ===== 模型管理 =====

  async getModels(): Promise<AIModelConfig[]> {
    try {
      return await invoke<AIModelConfig[]>('get_ai_models')
    } catch (error) {
      throw new Error(handleError(error, '获取AI模型失败'))
    }
  }

  async addModel(config: AIModelConfig): Promise<void> {
    try {
      await invoke('add_ai_model', { config })
    } catch (error) {
      throw new Error(handleError(error, '添加AI模型失败'))
    }
  }

  async updateModel(modelId: string, updates: Partial<AIModelConfig>): Promise<void> {
    try {
      await invoke('update_ai_model', { modelId, updates })
    } catch (error) {
      throw new Error(handleError(error, '更新AI模型失败'))
    }
  }

  async removeModel(modelId: string): Promise<void> {
    try {
      await invoke('remove_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '删除AI模型失败'))
    }
  }

  async setDefaultModel(modelId: string): Promise<void> {
    try {
      await invoke('set_default_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '设置默认模型失败'))
    }
  }

  async testConnection(modelId: string): Promise<boolean> {
    try {
      return await invoke('test_ai_connection', { modelId })
    } catch (error) {
      const errorMessage = handleError(error)
      if (errorMessage.includes('not implemented yet')) {
        return false
      }
      throw new Error(errorMessage)
    }
  }

  async testConnectionWithConfig(config: AIModelConfig): Promise<boolean> {
    try {
      return await invoke('test_ai_connection_with_config', { config })
    } catch (error) {
      throw new Error(handleError(error))
    }
  }

  // ===== 设置管理 =====

  async getSettings(): Promise<AISettings> {
    try {
      return await invoke('get_ai_settings')
    } catch (error) {
      throw new Error(handleError(error, '获取AI设置失败'))
    }
  }

  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    try {
      await invoke('update_ai_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, '更新AI设置失败'))
    }
  }

  // ===== 统计监控 =====

  async getStats(): Promise<AIStats> {
    try {
      return await invoke('get_ai_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取AI统计失败'))
    }
  }

  async getHealthStatus(): Promise<AIHealthStatus[]> {
    try {
      return await invoke('get_ai_health_status')
    } catch (error) {
      throw new Error(handleError(error, '获取AI健康状态失败'))
    }
  }

  // ===== 前置提示词 =====

  async getUserPrefixPrompt(): Promise<string | null> {
    try {
      return await invoke('get_user_prefix_prompt')
    } catch (error) {
      throw new Error(handleError(error, '获取用户前置提示词失败'))
    }
  }

  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    try {
      await invoke('set_user_prefix_prompt', { prompt })
    } catch (error) {
      throw new Error(handleError(error, '设置用户前置提示词失败'))
    }
  }

  // ===== 会话管理 =====

  get conversations() {
    return conversationAPI
  }

  // ===== 工具调用 =====

  async analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
    return analyzeCode(params)
  }

  async webFetch(request: WebFetchRequest): Promise<WebFetchResponse> {
    return webFetchHeadless(request)
  }
}

// 导出单例实例
export const aiApi = new AiApi()

// 导出类型
export type * from './tool'
export type { Conversation, Message } from '@/types/features/ai/chat'

// 默认导出
export default aiApi
