import { aiApi } from '@/api'

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AIModelConfig, AISettings } from '@/types'
import type { AIModelCreateInput, AIModelUpdateInput } from '@/api/ai/types'

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
    isLoading.value = false
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

    const updatedSettings = { ...settings.value, ...newSettings }
    settings.value = updatedSettings
    isLoading.value = false
  }

  const addModel = async (model: AIModelConfig) => {
    const payload: AIModelCreateInput = {
      name: model.name,
      provider: model.provider,
      apiUrl: model.apiUrl,
      apiKey: model.apiKey,
      model: model.model,
      modelType: model.modelType,
      enabled: model.enabled,
      options: model.options,
    }

    await aiApi.addModel(payload)
    await loadModels()
  }

  const updateModel = async (modelId: string, updates: Partial<AIModelConfig>) => {
    const existingModel = models.value.find(m => m.id === modelId)
    if (!existingModel) {
      throw new Error(`模型 ${modelId} 不存在`)
    }

    const updatedModel = { ...existingModel, ...updates }
    const payload: AIModelUpdateInput = {
      id: modelId,
      changes: {
        name: updatedModel.name,
        provider: updatedModel.provider,
        apiUrl: updatedModel.apiUrl,
        apiKey: updatedModel.apiKey,
        model: updatedModel.model,
        modelType: updatedModel.modelType,
        enabled: updatedModel.enabled,
        options: updatedModel.options,
      },
    }

    await aiApi.updateModel(payload)
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
