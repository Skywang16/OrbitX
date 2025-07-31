/**
 * 配置管理组合式 API
 *
 * 提供响应式的配置管理功能，包括配置获取、更新、重载等。
 */

import { computed, readonly, ref } from 'vue'
import {
  getConfig,
  getConfigFileInfo,
  getConfigFilePath,
  openConfigFile,
  resetConfigToDefaults,
  saveConfig,
  updateConfig,
  validateConfig,
} from '../api/config'
import { type AppConfig, type ConfigFileInfo, ConfigApiError } from '../components/settings/components/Config/types'

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
    return new Date(timestamp).toLocaleString()
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

  // 加载配置
  const loadConfig = async () => {
    loadingState.value.loading = true
    loadingState.value.error = null

    try {
      const loadedConfig = await getConfig()
      config.value = loadedConfig
      loadingState.value.lastUpdated = new Date()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('加载配置失败:', error)
    } finally {
      loadingState.value.loading = false
    }
  }

  // 更新配置
  const updateConfigData = async (newConfig: AppConfig) => {
    if (!config.value) {
      throw new Error('配置未加载')
    }

    loadingState.value.loading = true
    loadingState.value.error = null

    try {
      await updateConfig(newConfig)
      config.value = newConfig
      loadingState.value.lastUpdated = new Date()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('更新配置失败:', error)
      throw error
    } finally {
      loadingState.value.loading = false
    }
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
    loadingState.value.loading = true
    loadingState.value.error = null

    try {
      await saveConfig()
      loadingState.value.lastUpdated = new Date()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('保存配置失败:', error)
      throw error
    } finally {
      loadingState.value.loading = false
    }
  }

  // 验证配置
  const validateConfigData = async () => {
    loadingState.value.loading = true
    loadingState.value.error = null

    try {
      await validateConfig()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('验证配置失败:', error)
      throw error
    } finally {
      loadingState.value.loading = false
    }
  }

  // 重置为默认值
  const resetToDefaults = async () => {
    loadingState.value.loading = true
    loadingState.value.error = null

    try {
      await resetConfigToDefaults()
      await loadConfig() // 重新加载配置
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      loadingState.value.error = message
      console.error('重置配置失败:', error)
      throw error
    } finally {
      loadingState.value.loading = false
    }
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
  const fileReadable = computed(() => fileState.value.info?.readable ?? false)
  const fileWritable = computed(() => fileState.value.info?.writable ?? false)
  const fileSize = computed(() => formatFileSize(fileState.value.info?.size))
  const fileModifiedAt = computed(() => formatTimestamp(fileState.value.info?.modifiedAt))

  // 获取配置文件路径
  const getFilePath = async () => {
    try {
      const path = await getConfigFilePath()
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
      const info = await getConfigFileInfo()
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
      await openConfigFile()
    } catch (error) {
      const message = error instanceof ConfigApiError ? error.message : String(error)
      fileState.value.error = message
      console.error('打开配置文件失败:', error)
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

/**
 * 配置主题管理组合函数
 *
 * 提供基于配置系统的主题管理功能
 */
export const useConfigTheme = () => {
  const { config, updateConfigSection } = useConfig()

  // 主题相关状态
  const themeList = ref<Array<{ name: string; type: string }>>([])
  const themeLoading = ref(false)
  const themeError = ref<string | null>(null)
  const systemThemeListener = ref<(() => void) | null>(null)

  // 当前主题名称
  const currentThemeName = computed(() => config.value?.appearance?.theme_config?.terminal_theme || 'dark')

  // 更新主题
  const updateTheme = async (themeName: string) => {
    if (!config.value) {
      throw new Error('配置未加载')
    }

    await updateConfigSection('appearance', {
      theme_config: {
        ...config.value.appearance.theme_config,
        terminal_theme: themeName,
      },
    })
  }

  // 加载主题列表
  const loadThemeList = async () => {
    themeLoading.value = true
    themeError.value = null
    try {
      // 使用动态导入避免循环依赖
      const { getAvailableThemes } = await import('@/api/config/theme')
      const themes = await getAvailableThemes()
      themeList.value = themes.map(theme => ({
        name: theme.name,
        type: theme.themeType,
      }))
    } catch (err) {
      themeError.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      themeLoading.value = false
    }
  }

  // 切换主题
  const switchTheme = async (themeName: string) => {
    try {
      const { setTerminalTheme } = await import('@/api/config/theme')
      await setTerminalTheme(themeName)
      await updateTheme(themeName)
    } catch (err) {
      themeError.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 启用系统主题监听
  const enableSystemThemeWatch = async () => {
    try {
      // 动态导入避免循环依赖
      const { listen } = await import('@tauri-apps/api/event')

      // 监听主题变化事件
      const unlisten = await listen<string>('theme-changed', async event => {
        console.log('检测到主题变化:', event.payload)
        // 重新加载主题列表
        await loadThemeList().catch(console.error)

        // 应用主题到 UI
        try {
          const { getCurrentTheme } = await import('@/api/config/theme')
          const { applyThemeToUI } = await import('@/utils/themeApplier')
          const theme = await getCurrentTheme()
          applyThemeToUI(theme)
        } catch (error) {
          console.warn('应用主题到UI失败:', error)
        }
      })

      systemThemeListener.value = unlisten
      console.log('系统主题监听已启用')
    } catch (error) {
      console.warn('启用系统主题监听失败:', error)
      themeError.value = error instanceof Error ? error.message : String(error)
    }
  }

  // 禁用系统主题监听
  const disableSystemThemeWatch = () => {
    if (systemThemeListener.value) {
      systemThemeListener.value()
      systemThemeListener.value = null
      console.log('系统主题监听已禁用')
    }
  }

  // 清除错误
  const clearError = () => {
    themeError.value = null
  }

  return {
    // 状态
    currentThemeName: readonly(currentThemeName),
    themeList: readonly(themeList),
    themeLoading: readonly(themeLoading),
    themeError: readonly(themeError),

    // 方法
    updateTheme,
    loadThemeList,
    switchTheme,
    enableSystemThemeWatch,
    disableSystemThemeWatch,
    clearError,
  }
}

// ============================================================================
// 导出所有组合函数
// ============================================================================

export default {
  useConfig,
  useConfigFile,
  useConfigTheme,
}
