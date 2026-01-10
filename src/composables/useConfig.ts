/**
 * 配置管理组合式 API
 *
 * 提供响应式的配置管理功能，包括配置获取、更新、重载等。
 */

import { computed, readonly, ref } from 'vue'
import { configApi } from '@/api'
import { type AppConfig } from '@/api/config'

/**
 * 配置加载状态
 */
export interface ConfigLoadingState {
  loading: boolean
  error: string | null
  lastUpdated: Date | null
}

/**
 * 配置管理主要组合函数
 */
export const useConfig = () => {
  // 响应式状态
  const config = ref<AppConfig | null>(null)
  const loadingState = ref<ConfigLoadingState>({
    loading: false,
    error: null,
    lastUpdated: null,
  })

  const isLoaded = computed(() => config.value !== null)
  const hasError = computed(() => loadingState.value.error !== null)
  const isLoading = computed(() => loadingState.value.loading)

  // 通用异步操作处理
  const withLoading = async <T>(operation: () => Promise<T>): Promise<T> => {
    loadingState.value.loading = true
    loadingState.value.error = null
    try {
      const result = await operation()
      loadingState.value.lastUpdated = new Date()
      return result
    } finally {
      loadingState.value.loading = false
    }
  }

  // 加载配置
  const loadConfig = async () => {
    return withLoading(async () => {
      const loadedConfig = await configApi.getConfig()
      config.value = loadedConfig
      return loadedConfig
    })
  }

  // 更新配置
  const updateConfigData = async (newConfig: AppConfig) => {
    if (!config.value) {
      throw new Error('配置未加载')
    }

    return withLoading(async () => {
      await configApi.setConfig(newConfig)
      config.value = newConfig
    })
  }

  // 更新配置的特定部分
  const updateConfigSection = async <K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>) => {
    if (!config.value) {
      throw new Error('配置未加载')
    }

    const updatedConfig = {
      ...config.value,
      [section]: {
        ...(config.value[section] as object),
        ...(updates as object),
      },
    }

    await updateConfigData(updatedConfig)
  }

  // 重置为默认值
  const resetToDefaults = async () => {
    return withLoading(async () => {
      await configApi.resetToDefaults()
      await loadConfig() // 重新加载配置
    })
  }

  // 清除错误
  const clearError = () => {
    loadingState.value.error = null
  }

  // 初始化方法（需要手动调用）
  const initialize = async () => {
    await loadConfig()
  }

  return {
    config: readonly(config),
    loadingState: readonly(loadingState),

    isLoaded,
    hasError,
    isLoading,

    // 方法
    initialize,
    loadConfig,
    updateConfig: updateConfigData,
    updateConfigSection,
    resetToDefaults,
    clearError,
  }
}

export default {
  useConfig,
}
