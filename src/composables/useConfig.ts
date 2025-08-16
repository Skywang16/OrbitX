/**
 * 配置管理组合式 API
 *
 * 提供响应式的配置管理功能，包括配置获取、更新、重载等。
 */

import { computed, readonly, ref } from 'vue'
import { formatLocaleDateTime } from '@/utils/dateFormatter'
import { configApi } from '@/api'
import { type AppConfig, type ConfigFileInfo, ConfigApiError } from '@/api'

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 格式化文件大小
 */
const formatFileSize = (size?: number): string => {
  if (!size) return '未知'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}

/**
 * 格式化时间戳
 */
const formatTimestamp = (timestamp?: string): string => {
  if (!timestamp) return '未知'
  try {
    return formatLocaleDateTime(timestamp)
  } catch {
    return '无效时间'
  }
}

// ============================================================================
// 类型定义
// ============================================================================

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

// ============================================================================
// 主要配置管理组合函数
// ============================================================================

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
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('配置操作失败:', error)
      throw error
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
    try {
      await loadConfig()
    } catch (error) {
      console.error('初始化配置失败:', error)
    }
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

// ============================================================================
// 配置文件管理组合函数
// ============================================================================

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
  const fileReadable = computed(() => true) // 简化处理
  const fileWritable = computed(() => true) // 简化处理
  const fileSize = computed(() => formatFileSize(0)) // 简化处理
  const fileModifiedAt = computed(() => formatTimestamp(fileState.value.info?.lastModified?.toString()))

  // 获取配置文件路径
  const getFilePath = async () => {
    try {
      const path = await configApi.getFilePath()
      filePath.value = path
      return path
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      fileState.value.error = message
      console.error('获取配置文件路径失败:', error)
      throw error
    }
  }

  // 获取配置文件信息
  const getFileInfo = async () => {
    fileState.value.loading = true
    fileState.value.error = null

    try {
      const info = await configApi.getFileInfo()
      fileState.value.info = info
      return info
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      fileState.value.error = message
      console.error('获取配置文件信息失败:', error)
      throw error
    } finally {
      fileState.value.loading = false
    }
  }

  // 打开配置文件
  const openFile = async () => {
    try {
      await configApi.openFile()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      fileState.value.error = message

      throw error
    }
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
    fileReadable,
    fileWritable,
    fileSize,
    fileModifiedAt,

    // 方法
    initialize,
    getFilePath,
    getFileInfo,
    openFile,
    clearFileError,
  }
}

// ============================================================================
// 配置主题管理组合函数
// ============================================================================

// ============================================================================
// 导出所有组合函数
// ============================================================================

export default {
  useConfig,
  useConfigFile,
}
