/**
 * AI设置相关的状态管理
 *
 * 使用新的统一配置系统管理 AI 设置
 */

import { ai } from '@/api/ai'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AIModelConfig, AISettings } from './types'

// 默认AI设置配置
// 定义AI功能的默认配置项，包括模型、功能开关等
const DEFAULT_AI_SETTINGS: AISettings = {
  models: [],
  defaultModelId: null,
  features: {
    chat: {
      enabled: true,
      model: 'gpt-4',
      explanation: true,
      maxHistoryLength: Number.MAX_SAFE_INTEGER, // 无限历史长度
      autoSaveHistory: true,
      contextWindowSize: 4000,
    },
  },
  performance: {
    requestTimeout: 30,
    maxConcurrentRequests: 5,
    cacheEnabled: true,
    cacheTtl: 3600,
  },
}

export const useAISettingsStore = defineStore('ai-settings', () => {
  // ===== 状态定义 =====
  const settings = ref<AISettings>({ ...DEFAULT_AI_SETTINGS }) // AI设置配置
  const isLoading = ref(false) // 加载状态标志
  const error = ref<string | null>(null) // 错误信息

  // ===== 计算属性 =====
  // 获取当前默认模型配置
  const defaultModel = computed(() => {
    if (!settings.value.defaultModelId) return null
    return settings.value.models.find(m => m.id === settings.value.defaultModelId) || null
  })

  // 检查是否有配置的模型
  const hasModels = computed(() => {
    return settings.value.models.length > 0
  })

  // 获取所有启用的模型（目前所有配置的模型都被认为是启用的）
  const enabledModels = computed(() => {
    return settings.value.models
  })

  // ===== 操作方法 =====
  /**
   * 加载AI设置
   * 从 AI API 获取模型列表
   */
  const loadSettings = async () => {
    isLoading.value = true
    error.value = null

    try {
      // 先从localStorage加载缓存的设置
      const saved = localStorage.getItem('ai-settings')
      if (saved) {
        const parsedSettings = JSON.parse(saved)
        // 合并默认设置和保存的设置，确保结构完整
        settings.value = {
          ...DEFAULT_AI_SETTINGS,
          ...parsedSettings,
          features: {
            ...DEFAULT_AI_SETTINGS.features,
            ...parsedSettings.features,
            chat: { ...DEFAULT_AI_SETTINGS.features.chat, ...parsedSettings.features?.chat },
            explanation: { ...DEFAULT_AI_SETTINGS.features.explanation, ...parsedSettings.features?.explanation },
            errorAnalysis: {
              ...DEFAULT_AI_SETTINGS.features.errorAnalysis,
              ...parsedSettings.features?.errorAnalysis,
            },
          },
          performance: { ...DEFAULT_AI_SETTINGS.performance, ...parsedSettings.performance },
        }
      }

      // 然后从后端获取最新的模型列表
      const models = await ai.getModels()
      settings.value = {
        ...settings.value,
        models,
      }

      // 保存到本地存储
      saveToLocalStorage()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '加载AI设置失败')
      // 如果加载失败，使用默认设置
      settings.value = { ...DEFAULT_AI_SETTINGS }
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 保存设置到本地存储
   * 用于缓存设置，提高加载速度
   */
  const saveToLocalStorage = () => {
    try {
      localStorage.setItem('ai-settings', JSON.stringify(settings.value))
    } catch {
      // 本地存储失败不是致命错误，静默处理
    }
  }

  /**
   * 更新AI设置
   * @param newSettings 要更新的设置项（部分更新）
   */
  const updateSettings = async (newSettings: Partial<AISettings>) => {
    isLoading.value = true
    error.value = null

    try {
      // 合并新设置
      const updatedSettings = { ...settings.value, ...newSettings }

      // 更新本地状态
      settings.value = updatedSettings

      // 保存到本地存储
      saveToLocalStorage()
    } catch (err) {
      error.value = err instanceof Error ? err.message : '更新AI设置失败'
      throw err
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 添加AI模型配置
   * @param model 要添加的模型配置
   */
  const addModel = async (model: AIModelConfig) => {
    // 检查模型ID是否已存在
    if (settings.value.models.some(m => m.id === model.id)) {
      throw new Error(`模型 ID '${model.id}' 已存在`)
    }

    // 添加模型到当前设置
    const newModels = [...settings.value.models, model]
    await updateSettings({ models: newModels })
  }

  /**
   * 更新AI模型配置
   * @param modelId 要更新的模型ID
   * @param updates 要更新的配置项（部分更新）
   */
  const updateModel = async (modelId: string, updates: Partial<AIModelConfig>) => {
    const modelIndex = settings.value.models.findIndex(m => m.id === modelId)
    if (modelIndex === -1) {
      throw new Error(`模型 ID '${modelId}' 不存在`)
    }

    // 更新模型配置
    const newModels = [...settings.value.models]
    newModels[modelIndex] = { ...newModels[modelIndex], ...updates }
    await updateSettings({ models: newModels })
  }

  /**
   * 删除AI模型配置
   * @param modelId 要删除的模型ID
   */
  const removeModel = async (modelId: string) => {
    const modelExists = settings.value.models.some(m => m.id === modelId)
    if (!modelExists) {
      throw new Error(`模型 ID '${modelId}' 不存在`)
    }

    // 移除指定模型
    const newModels = settings.value.models.filter(m => m.id !== modelId)

    // 如果删除的是默认模型，清除默认设置
    const newDefaultModelId = settings.value.defaultModelId === modelId ? null : settings.value.defaultModelId

    await updateSettings({
      models: newModels,
      defaultModelId: newDefaultModelId,
    })
  }

  /**
   * 设置默认AI模型
   * @param modelId 要设为默认的模型ID，null表示清除默认模型
   */
  const setDefaultModel = async (modelId: string | null) => {
    const updates = { defaultModelId: modelId }
    await updateSettings(updates)
  }

  /**
   * 重置所有设置为默认值
   */
  const resetToDefaults = async () => {
    await updateSettings(DEFAULT_AI_SETTINGS)
  }

  /**
   * 清除错误信息
   */
  const clearError = () => {
    error.value = null
  }

  return {
    // ===== 状态 =====
    settings, // AI设置配置
    isLoading, // 加载状态
    error, // 错误信息

    // ===== 计算属性 =====
    defaultModel, // 当前默认模型
    hasModels, // 是否有配置的模型
    enabledModels, // 所有启用的模型

    // ===== 方法 =====
    loadSettings, // 加载设置
    updateSettings, // 更新设置
    addModel, // 添加模型
    updateModel, // 更新模型
    removeModel, // 删除模型
    setDefaultModel, // 设置默认模型
    resetToDefaults, // 重置为默认设置
    clearError, // 清除错误
  }
})
