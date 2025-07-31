/**
 * 设置相关的状态管理入口
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useConfigTheme } from '../../composables/useConfig'
import { useAISettingsStore } from './components/AI'

export const useSettingsStore = defineStore('settings', () => {
  // 设置页面状态
  const isSettingsOpen = ref(false)
  const activeSection = ref<string>('theme')

  // 子stores和组合函数
  const aiSettings = useAISettingsStore()
  const configTheme = useConfigTheme()

  // 操作方法
  const openSettings = () => {
    isSettingsOpen.value = true
  }

  const closeSettings = () => {
    isSettingsOpen.value = false
  }

  const setActiveSection = (section: string) => {
    activeSection.value = section
  }

  // 初始化所有设置
  const initializeSettings = async () => {
    try {
      // 加载主题列表
      await configTheme.loadThemeList()

      // 启用系统主题监听
      configTheme.enableSystemThemeWatch()

      // 加载AI设置
      await aiSettings.loadSettings()
    } catch (error) {
      console.error('初始化设置失败:', error)
    }
  }

  // 重置所有设置
  const resetAllSettings = async () => {
    try {
      await aiSettings.resetToDefaults()
      // 主题重置通过配置系统处理
      await configTheme.switchTheme('dark') // 重置为默认主题
    } catch (error) {
      console.error('重置设置失败:', error)
      throw error
    }
  }

  return {
    // 状态
    isSettingsOpen,
    activeSection,

    // 子stores和组合函数
    aiSettings,
    configTheme,

    // 方法
    openSettings,
    closeSettings,
    setActiveSection,
    initializeSettings,
    resetAllSettings,
  }
})

// 重新导出子stores
export { useAISettingsStore } from './components/AI'
