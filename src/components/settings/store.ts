/**
 * 设置相关的状态管理入口
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useTheme } from '../../composables/useTheme'
import { useAISettingsStore } from './components/AI'

export const useSettingsStore = defineStore('settings', () => {
  // 设置页面状态
  const activeSection = ref<string>('ai')

  // 子stores和组合函数
  const aiSettings = useAISettingsStore()
  const themeManager = useTheme()

  const setActiveSection = (section: string) => {
    activeSection.value = section
  }

  // 初始化所有设置
  const initializeSettings = async () => {
    try {
      // 初始化主题系统
      await themeManager.initialize()
      // 加载AI设置
      await aiSettings.loadSettings()
    } catch (error) {
      console.error('Failed to initialize settings:', error)
    }
  }

  // 重置所有设置
  const resetAllSettings = async () => {
    await aiSettings.resetToDefaults()
    // 主题重置为默认主题
    await themeManager.switchToTheme('dark')
  }

  return {
    // 状态
    activeSection,

    // 子stores和组合函数
    aiSettings,
    themeManager,

    // 方法
    setActiveSection,
    initializeSettings,
    resetAllSettings,
  }
})

// 重新导出子stores
export { useAISettingsStore } from './components/AI'
