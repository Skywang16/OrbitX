import { aiApi } from '@/api'

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AIModelConfig, AISettings } from '@/types'

export const useAISettingsStore = defineStore('ai-settings', () => {
  const settings = ref<AISettings | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const dataVersion = ref(0)
  const isInitialized = ref(false)

  const hasModels = computed(() => {
    return (settings.value?.models?.length || 0) > 0
  })

  const enabledModels = computed(() => {
    return settings.value?.models || []
  })

  const models = computed(() => {
    return settings.value?.models || []
  })

  const chatModels = computed(() => {
    return models.value.filter(model => model.modelType === 'chat')
  })

  const embeddingModels = computed(() => {
    return models.value.filter(model => model.modelType === 'embedding')
  })

  const loadModels = async () => {
    try {
      isLoading.value = true
      const models = await aiApi.getModels()

      if (!settings.value) {
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
      } else {
        settings.value.models = models
      }

      dataVersion.value++
    } catch (err) {
      error.value = 'Failed to load models'
      throw err
    } finally {
      isLoading.value = false
    }
  }

  const loadSettings = async (forceRefresh = false) => {
    if (isInitialized.value && !forceRefresh) return

    await loadModels()
    isInitialized.value = true
  }

  const updateSettings = async (newSettings: Partial<AISettings>) => {
    if (!settings.value) {
      throw new Error('AI设置未初始化')
    }

    isLoading.value = true
    error.value = null

    try {
      const updatedSettings = { ...settings.value, ...newSettings }
      settings.value = updatedSettings
    } catch (err) {
      error.value = err instanceof Error ? err.message : '更新AI设置失败'
      throw err
    } finally {
      isLoading.value = false
    }
  }

  const addModel = async (model: AIModelConfig) => {
    try {
      await aiApi.addModel(model)
      await loadModels()
    } catch (error) {
      console.error('模型添加失败:', error)
      throw error
    }
  }

  const updateModel = async (modelId: string, updates: Partial<AIModelConfig>) => {
    const existingModel = models.value.find(m => m.id === modelId)
    if (!existingModel) {
      throw new Error(`模型 ${modelId} 不存在`)
    }

    const updatedModel = { ...existingModel, ...updates }
    await aiApi.updateModel(updatedModel)
    await loadModels()
  }

  const removeModel = async (modelId: string) => {
    await aiApi.deleteModel(modelId)
    await loadModels()
  }

  const resetToDefaults = async () => {
    throw new Error('重置功能待实现 - 需要后端API支持')
  }

  const clearError = () => {
    error.value = null
  }

  return {
    settings,
    isLoading,
    error,
    dataVersion,
    isInitialized,
    hasModels,
    enabledModels,
    models,
    chatModels,
    embeddingModels,
    loadSettings,
    loadModels,
    updateSettings,
    addModel,
    updateModel,
    removeModel,
    resetToDefaults,
    clearError,
  }
})
