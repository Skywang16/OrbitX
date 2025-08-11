/**
 * 设置相关的状态管理入口
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useTheme } from '../../composables/useTheme'
import { useAISettingsStore } from './components/AI'

export const useSettingsStore = defineStore('settings', () => {
  // 设置页面状态
  const isSettingsOpen = ref(false)
  const activeSection = ref<string>('theme')

  // 子stores和组合函数
  const aiSettings = useAISettingsStore()
  const themeManager = useTheme()

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
      // 初始化主题系统
      await themeManager.initialize()
      // 加载AI设置
      await aiSettings.loadSettings()
    } catch (error) {
      // 统一错误提示
      // 使用懒加载 UI 层提示，避免打断初始化流程
    }
  }

  // 重置所有设置
  const resetAllSettings = async () => {
    try {
      await aiSettings.resetToDefaults()
      // 主题重置为默认主题
      await themeManager.switchToTheme('dark')
    } catch (error) {
      throw error
    }
  }

  return {
    // 状态
    isSettingsOpen,
    activeSection,

    // 子stores和组合函数
    aiSettings,
    themeManager,

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
