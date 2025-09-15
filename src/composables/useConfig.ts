/**
 * 配置管理组合式 API
 *
 * 提供响应式的配置管理功能，包括配置获取、更新、重载等。
 */

import { computed, readonly, ref } from 'vue'
import { formatLocaleDateTime } from '@/utils/dateFormatter'
import { configApi } from '@/api'
import { type AppConfig, type ConfigFileInfo } from '@/api/config'
import { useI18n } from 'vue-i18n'

/**
 * 格式化时间戳
 */
const formatTimestamp = (timestamp?: string): string => {
  if (!timestamp) return useI18n().t('config.unknown_time')
  try {
    return formatLocaleDateTime(timestamp)
  } catch {
    return useI18n().t('config.invalid_time')
  }
}

/**
 * 配置加载状态
 */
export interface ConfigLoadingState {
  loading: boolean
  error: string | null
  lastUpdated: Date | null
}

/**
 * 配置文件状态
 */
export interface ConfigFileState {
  info: ConfigFileInfo | null
  loading: boolean
  error: string | null
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

  // 计算属性
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
      await configApi.updateConfig(newConfig)
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

  // 保存配置
  const saveConfigData = async () => {
    return withLoading(async () => {
      await configApi.saveConfig()
    })
  }

  // 验证配置
  const validateConfigData = async () => {
    return withLoading(async () => {
      await configApi.validateConfig()
    })
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
    // 状态
    config: readonly(config),
    loadingState: readonly(loadingState),

    // 计算属性
    isLoaded,
    hasError,
    isLoading,

    // 方法
    initialize,
    loadConfig,
    updateConfig: updateConfigData,
    updateConfigSection,
    saveConfig: saveConfigData,
    validateConfig: validateConfigData,
    resetToDefaults,
    clearError,
  }
}

/**
 * 配置文件管理组合函数
 */
export const useConfigFile = () => {
  const fileState = ref<ConfigFileState>({
    info: null,
    loading: false,
    error: null,
  })

  const filePath = ref<string>('')

  // 计算属性
  const fileExists = computed(() => fileState.value.info?.exists ?? false)
  const fileModifiedAt = computed(() => formatTimestamp(fileState.value.info?.lastModified?.toString()))

  // 获取配置文件路径
  const getFilePath = async () => {
    const path = await configApi.getFilePath()
    filePath.value = path
    return path
  }

  // 获取配置文件信息
  const getFileInfo = async () => {
    fileState.value.loading = true
    fileState.value.error = null
    try {
      const info = await configApi.getFileInfo()
      fileState.value.info = info
      return info
    } finally {
      fileState.value.loading = false
    }
  }

  // 打开配置文件
  const openFile = async () => {
    await configApi.openFile()
  }

  // 清除文件错误
  const clearFileError = () => {
    fileState.value.error = null
  }

  // 初始化方法（需要手动调用）
  const initialize = () => {
    getFilePath()
    getFileInfo()
  }

  return {
    // 状态
    fileState: readonly(fileState),
    filePath: readonly(filePath),

    // 计算属性
    fileExists,
    fileModifiedAt,

    // 方法
    initialize,
    getFilePath,
    getFileInfo,
    openFile,
    clearFileError,
  }
}

export default {
  useConfig,
  useConfigFile,
}
