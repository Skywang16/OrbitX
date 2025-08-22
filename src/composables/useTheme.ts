/**
 * 主题管理组合式 API
 *
 * 提供响应式的主题管理功能，支持手动选择主题和跟随系统主题两种模式。
 */

import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { computed, readonly, ref } from 'vue'
import { themeAPI } from '@/api/config'
import type { Theme, ThemeConfigStatus, ThemeInfo, ThemeOption } from '@/types/domain/theme'
import { applyThemeToUI } from '../utils/themeApplier'

// ============================================================================
// 主题管理 Composable
// ============================================================================

export const useTheme = () => {
  // 状态管理
  const configStatus = ref<ThemeConfigStatus | null>(null)
  const currentThemeData = ref<Theme | null>(null)
  const availableThemes = ref<ThemeInfo[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // 事件监听器
  let themeChangeUnlisten: UnlistenFn | null = null

  // 计算属性
  const themeConfig = computed(() => configStatus.value?.themeConfig || null)
  const currentThemeName = computed(() => configStatus.value?.currentThemeName || '')
  const isSystemDark = computed(() => configStatus.value?.isSystemDark)
  const isFollowingSystem = computed(() => themeConfig.value?.followSystem || false)

  // 主题选项（用于UI显示）
  const themeOptions = computed((): ThemeOption[] => {
    return availableThemes.value.map(theme => ({
      value: theme.name,
      label: theme.name, // 直接使用主题名称作为显示标签
      type: theme.themeType,
      isCurrent: theme.isCurrent,
    }))
  })

  // ============================================================================
  // 核心方法
  // ============================================================================

  /**
   * 加载主题配置状态
   */
  const loadThemeConfigStatus = async () => {
    loading.value = true
    error.value = null

    try {
      const status = await themeAPI.getThemeConfigStatus()
      configStatus.value = status
      availableThemes.value = status.availableThemes
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 加载当前主题数据
   */
  const loadCurrentTheme = async () => {
    try {
      const theme = await themeAPI.getCurrentTheme()
      currentThemeData.value = theme

      // 应用主题到 UI
      applyThemeToUI(theme)

      return theme
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // ============================================================================
  // 主题切换方法
  // ============================================================================

  /**
   * 设置终端主题（手动模式）
   */
  const switchToTheme = async (themeName: string) => {
    loading.value = true
    error.value = null

    try {
      // 1. 更新配置
      await themeAPI.setTerminalTheme(themeName)
      // 2. 重新查询最新状态 - 简单的增删改查模式
      await loadThemeConfigStatus()
      await loadCurrentTheme()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 设置跟随系统主题
   */
  const setFollowSystem = async (followSystem: boolean, lightTheme?: string, darkTheme?: string) => {
    loading.value = true
    error.value = null

    try {
      // 1. 更新配置
      await themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
      // 2. 重新查询最新状态 - 简单的增删改查模式
      await loadThemeConfigStatus()
      await loadCurrentTheme()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 启用跟随系统主题
   */
  const enableFollowSystem = async (lightTheme: string, darkTheme: string) => {
    return setFollowSystem(true, lightTheme, darkTheme)
  }

  /**
   * 禁用跟随系统主题
   */
  const disableFollowSystem = async () => {
    return setFollowSystem(false)
  }

  // ============================================================================
  // 事件监听
  // ============================================================================

  /**
   * 监听主题变化事件（可选，用于外部主题变化同步）
   */
  const startThemeChangeListener = async () => {
    try {
      themeChangeUnlisten = await listen<string>('theme-changed', async _event => {
        // 如果需要响应外部主题变化，重新加载状态
        try {
          await loadThemeConfigStatus()
          await loadCurrentTheme()
        } catch (err) {
          console.error('外部事件同步失败:', err)
          error.value = err instanceof Error ? err.message : String(err)
        }
      })
    } catch (err) {
      console.warn('启动主题变化监听失败:', err)
    }
  }

  /**
   * 停止监听主题变化事件
   */
  const stopThemeChangeListener = () => {
    if (themeChangeUnlisten) {
      themeChangeUnlisten()
      themeChangeUnlisten = null
    }
  }

  // ============================================================================
  // 生命周期管理
  // ============================================================================

  /**
   * 初始化主题管理
   */
  const initialize = async () => {
    await loadThemeConfigStatus()
    await loadCurrentTheme()
    await startThemeChangeListener()
  }

  /**
   * 清理资源
   */
  const cleanup = () => {
    stopThemeChangeListener()
  }

  // ============================================================================
  // 返回接口
  // ============================================================================

  return {
    // 状态
    configStatus: readonly(configStatus),
    currentThemeData: readonly(currentThemeData),
    availableThemes: readonly(availableThemes),
    loading: readonly(loading),
    error: readonly(error),

    // 计算属性
    themeConfig: readonly(themeConfig),
    currentThemeName: readonly(currentThemeName),
    isSystemDark: readonly(isSystemDark),
    isFollowingSystem: readonly(isFollowingSystem),
    themeOptions: readonly(themeOptions),

    // 方法
    loadThemeConfigStatus,
    loadCurrentTheme,
    switchToTheme,
    setFollowSystem,
    enableFollowSystem,
    disableFollowSystem,
    initialize,
    cleanup,

    // 事件监听
    startThemeChangeListener,
    stopThemeChangeListener,
  }
}
