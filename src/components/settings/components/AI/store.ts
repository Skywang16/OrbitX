/**
 * AI设置相关的状态管理
 *
 * 使用新的统一存储系统管理 AI 设置
 */

import { aiApi } from '@/api'
import { handleErrorWithMessage } from '@/utils/errorHandler'

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AIModelConfig, AISettings } from '@/types'

export const useAISettingsStore = defineStore('ai-settings', () => {
  // ===== 状态定义 =====
  const settings = ref<AISettings | null>(null) // AI设置配置
  const isLoading = ref(false) // 加载状态标志
  const error = ref<string | null>(null) // 错误信息
  const dataVersion = ref(0) // 数据版本号，用于缓存控制
  const isInitialized = ref(false) // 是否已初始化

  // ===== 计算属性 =====
  // 检查是否有配置的模型
  const hasModels = computed(() => {
    return (settings.value?.models?.length || 0) > 0
  })

  // 获取所有启用的模型（目前所有配置的模型都被认为是启用的）
  const enabledModels = computed(() => {
    return settings.value?.models || []
  })

  // 获取模型列表（供AI设置页面使用）
  const models = computed(() => {
    return settings.value?.models || []
  })

  // ===== 操作方法 =====
  /**
   * 刷新模型数据
   * 强制从API重新获取模型列表
   */
  const refreshModels = async () => {
    try {
      const models = await aiApi.getModels()

      if (settings.value) {
        settings.value = {
          ...settings.value,
          models,
        }
      }

      dataVersion.value++
    } catch (err) {
      error.value = handleErrorWithMessage(err, '刷新模型列表失败')
      throw err
    }
  }

  /**
   * 加载AI设置
   * 从后端API获取完整的AI配置（已迁移到SQLite）
   */
  const loadSettings = async (forceRefresh = false) => {
    // 如果已经初始化且不是强制刷新，直接返回
    if (isInitialized.value && !forceRefresh) return

    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      // 从AI API获取模型数据
      const models = await aiApi.getModels()

      // 目前先构造一个基本的设置对象
      settings.value = {
        models,
        features: {
          chat: { enabled: true, maxHistoryLength: 1000, autoSaveHistory: true, contextWindowSize: 4000 },
        },
        performance: {
          requestTimeout: 30,
          maxConcurrentRequests: 5,
          cacheEnabled: true,
          cacheTtl: 3600,
        },
      } as AISettings

      dataVersion.value++
      isInitialized.value = true
    } catch (err) {
      error.value = handleErrorWithMessage(err, '加载AI设置失败')
      settings.value = null
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 更新AI设置
   * @param newSettings 要更新的设置项（部分更新）
   */
  const updateSettings = async (newSettings: Partial<AISettings>) => {
    if (!settings.value) {
      throw new Error('AI设置未初始化')
    }

    isLoading.value = true
    error.value = null

    try {
      // 合并新设置
      const updatedSettings = { ...settings.value, ...newSettings }

      // 乐观更新本地状态
      settings.value = updatedSettings
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
    // 直接使用AI API添加模型（存储到SQLite）
    await aiApi.addModel(model)

    // 刷新模型数据，确保所有组件同步更新
    await refreshModels()
  }

  /**
   * 更新AI模型配置
   * @param modelId 要更新的模型ID
   * @param updates 要更新的配置项（部分更新）
   */
  const updateModel = async (modelId: string, updates: Partial<AIModelConfig>) => {
    // 先获取现有模型配置
    const existingModel = models.value.find(m => m.id === modelId)
    if (!existingModel) {
      throw new Error(`模型 ${modelId} 不存在`)
    }

    // 合并更新
    const updatedModel = { ...existingModel, ...updates }

    // 直接使用AI API更新模型（存储到SQLite）
    await aiApi.updateModel(updatedModel)

    // 刷新模型数据，确保所有组件同步更新
    await refreshModels()
  }

  /**
   * 删除AI模型配置
   * @param modelId 要删除的模型ID
   */
  const removeModel = async (modelId: string) => {
    // 直接使用AI API删除模型（从SQLite中删除）
    await aiApi.deleteModel(modelId)

    // 刷新模型数据，确保所有组件同步更新
    await refreshModels()
  }

  /**
   * 重置所有设置为默认值
   */
  const resetToDefaults = async () => {
    // TODO: 实现从后端API获取默认设置并重置
    throw new Error('重置功能待实现 - 需要后端API支持')
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
    dataVersion, // 数据版本号
    isInitialized, // 是否已初始化

    // ===== 计算属性 =====
    hasModels, // 是否有配置的模型
    enabledModels, // 所有启用的模型
    models, // 模型列表（供AI设置页面使用）

    // ===== 方法 =====
    loadSettings, // 加载设置
    refreshModels, // 刷新模型数据
    updateSettings, // 更新设置
    addModel, // 添加模型
    updateModel, // 更新模型
    removeModel, // 删除模型
    resetToDefaults, // 重置为默认设置
    clearError, // 清除错误
  }
})
