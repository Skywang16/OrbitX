/**
 * Eko框架核心配置模块
 * 负责管理LLM配置、Agent配置和Eko实例初始化
 */

import type { LLMs } from '@eko-ai/eko'
import type { AIModelConfig } from '@/types'
import { aiApi } from '@/api'

/**
 * 将项目的AIModelConfig转换为Eko的LLM配置
 */
export const convertToEkoLLMConfig = (modelConfig: AIModelConfig) => {
  // 根据provider映射到eko支持的provider
  const providerMap: Record<string, 'openai' | 'anthropic'> = {
    openAI: 'openai',
    claude: 'anthropic',
    custom: 'openai', // 自定义provider默认使用openai格式
  }

  const ekoProvider = providerMap[modelConfig.provider] || 'openai'

  return {
    provider: ekoProvider,
    model: modelConfig.model,
    apiKey: modelConfig.apiKey,
    config: {
      baseURL: modelConfig.apiUrl,
      maxTokens: modelConfig.options?.maxTokens,
      temperature: modelConfig.options?.temperature,
      timeout: modelConfig.options?.timeout,
    },
  }
}

/**
 * 获取当前选中的模型配置并转换为Eko LLMs格式
 */
export const getEkoLLMsConfig = async (): Promise<LLMs> => {
  try {
    // 获取所有模型配置
    const models = await aiApi.getModels()

    if (models.length === 0) {
      throw new Error('没有配置任何AI模型，请先在设置中添加模型配置')
    }

    // 找到默认模型
    const defaultModel = models.find(model => model.isDefault) || models[0]
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
    throw error
  }
}

/**
 * 获取默认模型ID
 */
export const getDefaultModelId = async (): Promise<string> => {
  try {
    const models = await aiApi.getModels()
    const defaultModel = models.find(model => model.isDefault) || models[0]

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
 * 验证模型配置是否有效
 */
export const validateModelConfig = async (modelId?: string): Promise<boolean> => {
  try {
    if (modelId) {
      return await aiApi.testConnection(modelId)
    } else {
      const defaultModelId = await getDefaultModelId()
      return await aiApi.testConnection(defaultModelId)
    }
  } catch (error) {
    console.error('验证模型配置失败:', error)
    return false
  }
}

/**
 * Eko配置选项
 */
export interface EkoConfigOptions {
  /** 是否启用调试模式 */
  debug?: boolean
  /** 自定义模型ID */
  modelId?: string
  /** 最大重试次数 */
  maxRetries?: number
  /** 请求超时时间(毫秒) */
  timeout?: number
}

/**
 * 获取完整的Eko配置
 */
export const getEkoConfig = async (options: EkoConfigOptions = {}) => {
  const { debug = false, modelId, maxRetries = 3, timeout = 30000 } = options

  try {
    // 获取LLM配置
    const llms = await getEkoLLMsConfig()

    // 验证模型配置
    const isValid = await validateModelConfig(modelId)
    if (!isValid) {
      console.warn('模型配置验证失败，但继续使用')
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
    throw error
  }
}
