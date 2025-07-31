/**
 * 配置管理 Store
 */

import { getConfig, updateConfig, type Theme } from '@/api/config'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AppConfig } from './types'

export const useConfigStore = defineStore('config', () => {
  // 状态
  const config = ref<AppConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // 主题相关状态
  const themeList = ref<Array<{ name: string; type: string }>>([])
  const themeLoading = ref(false)
  const themeError = ref<string | null>(null)
  const loadedThemes = ref<Map<string, Theme>>(new Map())

  // 计算属性
  const isLoaded = computed(() => config.value !== null)
  const currentTheme = computed(() => config.value?.appearance.theme_config.dark_theme ?? 'dark')
  const availableThemes = computed(() => themeList.value)
  const currentThemeData = computed(() => {
    const themeName = currentTheme.value
    return loadedThemes.value.get(themeName) || null
  })

  // Actions
  const loadConfig = async () => {
    loading.value = true
    error.value = null
    try {
      config.value = await getConfig()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const updateConfigData = async (newConfig: AppConfig) => {
    loading.value = true
    error.value = null
    try {
      await updateConfig(newConfig)
      config.value = newConfig
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const updateTheme = async (themeName: string) => {
    if (!config.value) return
    const updatedConfig = {
      ...config.value,
      appearance: {
        ...config.value.appearance,
        theme_config: {
          ...config.value.appearance.theme_config,
          terminal_theme: themeName,
        },
      },
    }
    await updateConfigData(updatedConfig)
  }

  // 主题相关方法
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

  const loadThemeData = async (themeName: string) => {
    if (loadedThemes.value.has(themeName)) {
      return loadedThemes.value.get(themeName)!
    }

    try {
      const { getCurrentTheme } = await import('@/api/config/theme')
      const themeData = await getCurrentTheme()
      loadedThemes.value.set(themeName, themeData)
      return themeData
    } catch (err) {
      themeError.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  const switchToTheme = async (themeName: string) => {
    try {
      const { setTerminalTheme } = await import('@/api/config/theme')
      await setTerminalTheme(themeName)
      await updateTheme(themeName)
    } catch (err) {
      themeError.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  const clearError = () => {
    error.value = null
    themeError.value = null
  }

  return {
    // 基础状态
    config,
    loading,
    error,
    isLoaded,

    // 主题状态
    themeList,
    themeLoading,
    themeError,
    currentTheme,
    availableThemes,
    currentThemeData,

    // 基础方法
    loadConfig,
    updateConfig: updateConfigData,
    updateTheme,
    clearError,

    // 主题方法
    loadThemeList,
    loadThemeData,
    switchToTheme,
  }
})
