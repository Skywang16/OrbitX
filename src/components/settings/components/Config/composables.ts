/**
 * 配置管理组合式 API
 */

import { storeToRefs } from 'pinia'
import { useConfigStore } from './store'

/**
 * 配置管理组合函数
 */
export const useConfig = () => {
  const configStore = useConfigStore()
  const { config, loading, error, isLoaded, currentTheme } = storeToRefs(configStore)
  const { loadConfig, updateConfig, updateTheme, clearError } = configStore

  // 初始化方法（需要手动调用）
  const initialize = async () => {
    if (!isLoaded.value) {
      try {
        await loadConfig()
      } catch (err) {
        // 配置初始化失败，静默处理
      }
    }
  }

  return {
    config,
    loading,
    error,
    isLoaded,
    currentTheme,
    initialize,
    loadConfig,
    updateConfig,
    updateTheme,
    clearError,
  }
}
