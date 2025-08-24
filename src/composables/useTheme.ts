// 主题管理（单例）

import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { computed, readonly, ref } from 'vue'
import { themeAPI } from '@/api/config'
import type { Theme, ThemeConfigStatus, ThemeInfo, ThemeOption } from '@/types/domain/theme'
import { applyThemeToUI } from '../utils/themeApplier'

// 状态（全局共享）
const configStatus = ref<ThemeConfigStatus | null>(null)
const currentThemeData = ref<Theme | null>(null)
const availableThemes = ref<ThemeInfo[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
// 事件监听句柄（全局唯一）
let themeChangeUnlisten: UnlistenFn | null = null

// 计算属性
const themeConfig = computed(() => configStatus.value?.themeConfig || null)
const currentThemeName = computed(() => configStatus.value?.currentThemeName || '')
const isSystemDark = computed(() => configStatus.value?.isSystemDark)
const isFollowingSystem = computed(() => themeConfig.value?.followSystem || false)

// UI 选择项
const themeOptions = computed((): ThemeOption[] => {
  return availableThemes.value.map(theme => ({
    value: theme.name,
    label: theme.name,
    type: theme.themeType,
    isCurrent: theme.isCurrent,
  }))
})

export const useTheme = () => {
  // 加载主题配置
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

  // 加载当前主题并应用到 UI
  const loadCurrentTheme = async () => {
    try {
      const theme = await themeAPI.getCurrentTheme()
      // 先应用 CSS 变量，再发布状态，确保监听者读取到已生效的样式
      applyThemeToUI(theme)
      currentThemeData.value = theme

      return theme
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 手动切换主题
  const switchToTheme = async (themeName: string) => {
    loading.value = true
    error.value = null

    try {
      await themeAPI.setTerminalTheme(themeName)
      await loadThemeConfigStatus()
      await loadCurrentTheme()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const setFollowSystem = async (followSystem: boolean, lightTheme?: string, darkTheme?: string) => {
    loading.value = true
    error.value = null

    try {
      await themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
      await loadThemeConfigStatus()
      await loadCurrentTheme()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const enableFollowSystem = async (lightTheme: string, darkTheme: string) => {
    return setFollowSystem(true, lightTheme, darkTheme)
  }

  const disableFollowSystem = async () => {
    return setFollowSystem(false)
  }

  // 监听外部主题变化
  const startThemeChangeListener = async () => {
    try {
      if (themeChangeUnlisten) return
      themeChangeUnlisten = await listen<string>('theme-changed', async _event => {
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

  // 初始化
  const initialize = async () => {
    await loadThemeConfigStatus()
    await loadCurrentTheme()
    await startThemeChangeListener()
  }

  // 停止监听（用于应用退出时）
  const stopThemeChangeListener = () => {
    if (themeChangeUnlisten) {
      try {
        themeChangeUnlisten()
      } catch {
        /* noop */
      }
      themeChangeUnlisten = null
    }
  }

  // 清理（可在全局退出时调用）
  const cleanup = () => {
    stopThemeChangeListener()
  }

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
    startThemeChangeListener,
    stopThemeChangeListener,
    cleanup,
  }
}
