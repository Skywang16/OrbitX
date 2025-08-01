/**
 * 配置管理 Store
 *
 * 使用新的统一存储API管理配置
 */

import { getConfig, updateConfig, type Theme } from '@/api/config'
import { storage } from '@/api/storage'
import { ConfigSection } from '@/types/storage'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AppConfig } from './types'

export const useConfigStore = defineStore('config', () => {
  // 状态
  const config = ref<AppConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // 计算属性
  const isLoaded = computed(() => config.value !== null)
  const currentTheme = computed(() => config.value?.appearance.theme_config.dark_theme ?? 'dark')

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

  const clearError = () => {
    error.value = null
  }

  // 新的存储API方法
  const loadConfigWithNewAPI = async () => {
    loading.value = true
    error.value = null
    try {
      // 使用新的存储API获取完整配置
      const [appConfig, appearanceConfig, terminalConfig, shortcutsConfig] = await Promise.all([
        storage.getConfig(ConfigSection.App),
        storage.getConfig(ConfigSection.Appearance),
        storage.getConfig(ConfigSection.Terminal),
        storage.getConfig(ConfigSection.Shortcuts),
      ])

      // 组合成完整的配置对象
      config.value = {
        app: appConfig,
        appearance: appearanceConfig,
        terminal: terminalConfig,
        shortcuts: shortcutsConfig,
      } as AppConfig
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const updateConfigWithNewAPI = async (section: ConfigSection, data: any) => {
    loading.value = true
    error.value = null
    try {
      await storage.updateConfig(section, data)

      // 乐观更新本地状态
      if (config.value) {
        const sectionKey = section.valueOf() as keyof AppConfig
        config.value = {
          ...config.value,
          [sectionKey]: { ...config.value[sectionKey], ...data },
        }
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      // 如果更新失败，重新加载配置
      await loadConfigWithNewAPI()
      throw err
    } finally {
      loading.value = false
    }
  }

  return {
    // 基础状态
    config,
    loading,
    error,
    isLoaded,

    // 基础状态
    currentTheme,

    // 基础方法
    loadConfig,
    updateConfig: updateConfigData,
    updateTheme,
    clearError,
  }
})
