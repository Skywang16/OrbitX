/**
 * Eko框架核心配置模块
 * 负责管理LLM配置、Agent配置和Eko实例初始化
 */

import type { LLMs } from '@/eko-core'
import type { AIModelConfig } from '@/types'
import { aiApi } from '@/api'

/**
 * 将项目的AIModelConfig转换为Eko的LLM配置
 */
export const convertToEkoLLMConfig = (modelConfig: AIModelConfig) => {
  return {
    modelId: modelConfig.id, // 使用数据库ID而不是model名称
    temperature: modelConfig.options?.temperature,
    maxTokens: modelConfig.options?.maxTokens,
  }
}

/**
 * 获取当前选中的模型配置并转换为Eko LLMs格式
 */
export const getEkoLLMsConfig = async (selectedModelId?: string | null): Promise<LLMs | null> => {
  try {
    // 获取所有模型配置
    const models = await aiApi.getModels()

    if (models.length === 0) {
      console.warn('没有配置AI模型，Eko功能将不可用。请在设置中添加AI模型配置。')
      return null
    }

    // 根据用户选择的模型ID确定默认模型
    let defaultModel = models[0] // 默认使用第一个模型
    if (selectedModelId) {
      const selectedModel = models.find(m => m.id === selectedModelId)
      if (selectedModel) {
        defaultModel = selectedModel
      }
    }

    // 构建LLMs配置对象
    const llms: LLMs = {
      default: convertToEkoLLMConfig(defaultModel),
    }

    // 添加其他模型作为备选
    models.forEach(model => {
      if (model.id !== defaultModel.id) {
        llms[model.id] = convertToEkoLLMConfig(model)
      }
    })

    return llms
  } catch (error) {
    console.error('获取Eko LLMs配置失败:', error)
    return null
  }
}

/**
 * 获取默认模型ID
 */
export const getDefaultModelId = async (): Promise<string> => {
  try {
    const models = await aiApi.getModels()
    const defaultModel = models[0]

    if (!defaultModel) {
      throw new Error('没有找到可用的AI模型')
    }

    return defaultModel.id
  } catch (error) {
    console.error('获取默认模型ID失败:', error)
    throw error
  }
}

/**
 * Eko配置选项
 */
export interface EkoConfigOptions {
  /** 是否启用调试模式 */
  debug?: boolean
  /** 最大重试次数 */
  maxRetries?: number
  /** 请求超时时间(毫秒) */
  timeout?: number
  /** 选中的模型ID */
  selectedModelId?: string | null
}

/**
 * 获取完整的Eko配置
 */
export const getEkoConfig = async (options: EkoConfigOptions = {}) => {
  const { debug = false, maxRetries = 3, timeout = 120000, selectedModelId } = options

  try {
    // 获取LLM配置，传递选中的模型ID
    const llms = await getEkoLLMsConfig(selectedModelId)

    if (!llms) {
      // 如果没有AI模型配置，返回一个最小的配置，让应用能够启动
      return {
        llms: null,
        debug,
        maxRetries,
        timeout,
        planLlms: [],
      }
    }

    return {
      llms,
      debug,
      maxRetries,
      timeout,
      planLlms: ['default'], // 使用默认模型进行规划
    }
  } catch (error) {
    console.error('获取Eko配置失败:', error)
    // 即使出错也返回一个最小配置，而不是抛出异常
    return {
      llms: null,
      debug,
      maxRetries,
      timeout,
      planLlms: [],
    }
  }
}
