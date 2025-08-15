/**
 * AI功能相关的API接口 - 重构版本
 *
 * 集成新的会话上下文管理系统
 */

import type { AIHealthStatus, AIModelConfig, AISettings, AIStats } from '@/types'
import { invoke } from '@/utils/request'

import { handleError } from '../../utils/errorHandler'
// 导入新的会话管理API
import { conversationAPI, conversations } from './conversations'

/**
 * AI管理API类
 * 提供AI相关的功能和管理操作，包括模型管理、聊天、补全等功能
 */
export class AIAPI {
  // ===== AI配置管理 =====

  /**
   * 获取所有AI模型配置
   * @returns 返回所有已配置的AI模型列表
   */
  async getModels(): Promise<AIModelConfig[]> {
    try {
      return await invoke<AIModelConfig[]>('get_ai_models')
    } catch (error) {
      throw new Error(handleError(error, '获取AI模型失败'))
    }
  }

  /**
   * 添加AI模型配置
   * @param config AI模型配置对象，包含模型ID、名称、API密钥等信息
   */
  async addModel(config: AIModelConfig): Promise<void> {
    try {
      await invoke('add_ai_model', { config })
    } catch (error) {
      throw new Error(handleError(error, '添加AI模型失败'))
    }
  }

  /**
   * 更新AI模型配置
   * @param modelId 要更新的模型ID
   * @param updates 要更新的配置项（部分更新）
   */
  async updateModel(modelId: string, updates: Partial<AIModelConfig>): Promise<void> {
    try {
      await invoke('update_ai_model', { modelId, updates })
    } catch (error) {
      throw new Error(handleError(error, '更新AI模型失败'))
    }
  }

  /**
   * 删除AI模型配置
   * @param modelId 要删除的模型ID
   */
  async removeModel(modelId: string): Promise<void> {
    try {
      await invoke('remove_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '删除AI模型失败'))
    }
  }

  /**
   * 设置默认AI模型
   * @param modelId 要设置为默认的模型ID
   */
  async setDefaultModel(modelId: string): Promise<void> {
    try {
      await invoke('set_default_ai_model', { modelId })
    } catch (error) {
      throw new Error(handleError(error, '设置默认模型失败'))
    }
  }

  /**
   * 测试AI模型连接
   * @param modelId 要测试的模型ID
   * @returns 连接是否成功
   */
  async testConnection(modelId: string): Promise<boolean> {
    try {
      return await invoke('test_ai_connection', { modelId })
    } catch (error) {
      const errorMessage = handleError(error)
      // 处理未实现的适配器
      if (errorMessage.includes('not implemented yet')) {
        return false
      }
      throw new Error(errorMessage)
    }
  }

  /**
   * 测试AI模型连接（基于表单配置）
   * @param config AI模型配置对象
   * @returns 连接是否成功
   */
  async testConnectionWithConfig(config: AIModelConfig): Promise<boolean> {
    try {
      return await invoke('test_ai_connection_with_config', { config })
    } catch (error) {
      const errorMessage = handleError(error)
      throw new Error(errorMessage)
    }
  }

  // ===== AI功能接口 =====

  // ===== AI设置管理 =====

  /**
   * 获取AI设置配置
   * @returns 返回当前的AI设置配置
   */
  async getSettings(): Promise<AISettings> {
    try {
      return await invoke('get_ai_settings')
    } catch (error) {
      throw new Error(handleError(error, '获取AI设置失败'))
    }
  }

  /**
   * 更新AI设置配置
   * @param settings 要更新的设置项（部分更新）
   */
  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    try {
      return await invoke('update_ai_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, '更新AI设置失败'))
    }
  }

  // ===== 统计和监控 =====

  /**
   * 获取AI使用统计数据
   * @returns 返回AI功能的使用统计信息
   */
  async getStats(): Promise<AIStats> {
    try {
      return await invoke('get_ai_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取AI统计失败'))
    }
  }

  /**
   * 获取AI服务健康状态
   * @returns 返回所有AI模型的健康状态列表
   */
  async getHealthStatus(): Promise<AIHealthStatus[]> {
    try {
      return await invoke('get_ai_health_status')
    } catch (error) {
      throw new Error(handleError(error, '获取AI健康状态失败'))
    }
  }

  // ===== 用户前置提示词管理 =====

  /**
   * 获取用户前置提示词
   * @returns 返回用户设置的前置提示词，如果没有设置则返回null
   */
  async getUserPrefixPrompt(): Promise<string | null> {
    try {
      return await invoke('get_user_prefix_prompt')
    } catch (error) {
      throw new Error(handleError(error, '获取用户前置提示词失败'))
    }
  }

  /**
   * 设置用户前置提示词
   * @param prompt 要设置的前置提示词，传入null表示清除
   */
  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    try {
      return await invoke('set_user_prefix_prompt', { prompt })
    } catch (error) {
      throw new Error(handleError(error, '设置用户前置提示词失败'))
    }
  }
}

/**
 * AI API单例实例
 * 全局唯一的AI API实例，用于所有AI相关操作
 */
export const aiAPI = new AIAPI()

/**
 * 便捷的AI操作函数集合 - 重构版本
 * 提供简化的AI功能调用接口，集成新的会话管理系统
 */
export const ai = {
  // 模型管理
  getModels: () => aiAPI.getModels(),
  addModel: (config: AIModelConfig) => aiAPI.addModel(config),
  updateModel: (modelId: string, updates: Partial<AIModelConfig>) => aiAPI.updateModel(modelId, updates),
  removeModel: (modelId: string) => aiAPI.removeModel(modelId),
  deleteModel: (modelId: string) => aiAPI.removeModel(modelId), // 别名
  setDefaultModel: (modelId: string) => aiAPI.setDefaultModel(modelId),
  testConnection: (modelId: string) => aiAPI.testConnection(modelId),
  testConnectionWithConfig: (config: AIModelConfig) => aiAPI.testConnectionWithConfig(config),

  // 新的会话管理功能
  conversations: {
    create: conversations.create,
    getList: conversations.getList,
    get: conversations.get,
    updateTitle: conversations.updateTitle,
    delete: conversations.delete,
    getCompressedContext: conversations.getCompressedContext,
    saveMessage: conversations.saveMessage,
    updateMessageMeta: (messageId: number, steps?: any[] | null, status?: string | null, duration?: number | null) =>
      conversationAPI.updateMessageMeta(messageId, steps, status as any, duration),
    truncateConversation: conversations.truncateConversation,
  },

  // 设置管理
  getSettings: () => aiAPI.getSettings(),
  updateSettings: (settings: Partial<AISettings>) => aiAPI.updateSettings(settings),

  // 统计监控
  getStats: () => aiAPI.getStats(),
  getHealthStatus: () => aiAPI.getHealthStatus(),

  // 用户前置提示词管理
  getUserPrefixPrompt: () => aiAPI.getUserPrefixPrompt(),
  setUserPrefixPrompt: (prompt: string | null) => aiAPI.setUserPrefixPrompt(prompt),
}

// 导出会话管理API
export { conversationAPI, conversations }

// 默认导出
export default aiAPI

// 类型定义现在统一从 @/types 导入，不在此处重复导出
