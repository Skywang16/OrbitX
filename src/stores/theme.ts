/**
 * 主题状态管理 Store
 * 使用 Pinia 实现集中化的主题状态管理
 */

import { themeAPI } from '@/api/config'
import type { Theme, ThemeConfigStatus } from '@/types/domain/theme'
import { applyThemeToUI } from '@/utils/themeApplier'
import { defineStore } from 'pinia'
import { computed, readonly, ref } from 'vue'

enum ThemeOperationState {
  IDLE = 'idle',
  SWITCHING = 'switching',
  UPDATING_CONFIG = 'updating_config',
  ERROR = 'error',
}

interface ThemeOperation {
  type: 'SWITCH_THEME' | 'SET_FOLLOW_SYSTEM' | 'LOAD_CONFIG'
  payload?: {
    themeName?: string
    followSystem?: boolean
    lightTheme?: string
    darkTheme?: string
  }
  timestamp: number
}

interface StateSnapshot {
  configStatus: ThemeConfigStatus | null
  currentTheme: Theme | null
}

export const useThemeStore = defineStore('theme', () => {
  const configStatus = ref<ThemeConfigStatus | null>(null)
  const currentTheme = ref<Theme | null>(null)
  const availableThemes = ref<Theme[]>([])

  const operationState = ref<ThemeOperationState>(ThemeOperationState.IDLE)
  const error = ref<string | null>(null)
  const lastOperation = ref<ThemeOperation | null>(null)

  const themeConfig = computed(() => configStatus.value?.themeConfig || null)
  const currentThemeName = computed(() => configStatus.value?.currentThemeName || '')
  const isSystemDark = computed(() => configStatus.value?.isSystemDark)
  const isFollowingSystem = computed(() => themeConfig.value?.followSystem || false)
  const isLoading = computed(() => operationState.value !== ThemeOperationState.IDLE)

  const themeOptions = computed(() => {
    const currentName = currentThemeName.value
    return availableThemes.value.map(theme => ({
      value: theme.name,
      label: theme.name,
      type: theme.themeType,
      isCurrent: theme.name === currentName,
      // 完整的主题数据用于预览
      ui: theme.ui,
    }))
  })

  // 核心业务逻辑

  /**
   * 主题切换核心逻辑
   * 使用状态机模式管理复杂的切换流程
   */
  class ThemeSwitcher {
    private store = {
      configStatus,
      currentTheme,
      operationState,
      error,
      lastOperation,
    }

    switchToTheme = async (themeName: string): Promise<void> => {
      const operation: ThemeOperation = {
        type: 'SWITCH_THEME',
        payload: { themeName },
        timestamp: Date.now(),
      }

      return this.executeOperation(operation, async () => {
        // 1. 先进行乐观更新，立即更新UI状态
        this.optimisticUpdateTheme(themeName)

        // 2. 调用后端API切换主题
        await themeAPI.setTerminalTheme(themeName)

        // 3. 获取最新的真实主题数据
        const newTheme = await themeAPI.getCurrentTheme()
        this.store.currentTheme.value = newTheme

        // 4. 应用真实主题到UI（覆盖乐观更新）
        applyThemeToUI(newTheme)
      })
    }

    setFollowSystem = async (followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> => {
      const operation: ThemeOperation = {
        type: 'SET_FOLLOW_SYSTEM',
        payload: { followSystem, lightTheme, darkTheme },
        timestamp: Date.now(),
      }

      return this.executeOperation(operation, async () => {
        // 1. 先进行乐观更新，立即更新配置状态
        this.optimisticUpdateFollowSystem(followSystem, lightTheme, darkTheme)

        // 2. 调用后端API同步配置
        await themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)

        // 3. 获取当前应该使用的主题数据
        const newTheme = await themeAPI.getCurrentTheme()
        this.store.currentTheme.value = newTheme

        // 4. 应用主题到UI
        applyThemeToUI(newTheme)
      })
    }

    private executeOperation = async (operation: ThemeOperation, action: () => Promise<void>): Promise<void> => {
      // 保存当前状态用于回滚
      const snapshot = this.createStateSnapshot()

      try {
        this.store.operationState.value = ThemeOperationState.SWITCHING
        this.store.error.value = null
        this.store.lastOperation.value = operation

        await action()

        this.store.operationState.value = ThemeOperationState.IDLE
      } catch (err) {
        // 回滚状态
        this.restoreStateSnapshot(snapshot)
        this.store.operationState.value = ThemeOperationState.ERROR
        this.store.error.value = err instanceof Error ? err.message : String(err)
        throw err
      }
    }

    private optimisticUpdateTheme = (themeName: string): void => {
      if (this.store.configStatus.value) {
        this.store.configStatus.value = {
          ...this.store.configStatus.value,
          currentThemeName: themeName,
          themeConfig: {
            ...this.store.configStatus.value.themeConfig,
            terminalTheme: themeName,
            followSystem: false,
          },
        }
      }
    }

    private optimisticUpdateFollowSystem = (followSystem: boolean, lightTheme?: string, darkTheme?: string): void => {
      if (this.store.configStatus.value) {
        this.store.configStatus.value = {
          ...this.store.configStatus.value,
          themeConfig: {
            ...this.store.configStatus.value.themeConfig,
            followSystem,
            ...(lightTheme && { lightTheme }),
            ...(darkTheme && { darkTheme }),
          },
        }
      }
    }

    private createStateSnapshot = (): StateSnapshot => {
      return {
        configStatus: this.store.configStatus.value,
        currentTheme: this.store.currentTheme.value,
      }
    }

    private restoreStateSnapshot = (snapshot: StateSnapshot): void => {
      this.store.configStatus.value = snapshot.configStatus
      this.store.currentTheme.value = snapshot.currentTheme

      // 如果有当前主题，需要重新应用到UI以回滚视觉效果
      if (snapshot.currentTheme) {
        applyThemeToUI(snapshot.currentTheme)
      }
    }
  }

  const themeSwitcher = new ThemeSwitcher()

  // 数据加载

  const loadThemeConfigStatus = async (): Promise<void> => {
    try {
      operationState.value = ThemeOperationState.UPDATING_CONFIG
      const [status, themes] = await Promise.all([themeAPI.getThemeConfigStatus(), themeAPI.getAvailableThemes()])
      configStatus.value = status
      availableThemes.value = themes
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      operationState.value = ThemeOperationState.IDLE
    }
  }

  const loadCurrentTheme = async (): Promise<void> => {
    try {
      const theme = await themeAPI.getCurrentTheme()
      currentTheme.value = theme

      // 应用主题到UI
      if (theme) {
        applyThemeToUI(theme)
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 初始化和清理

  const initialize = async (): Promise<void> => {
    await Promise.all([loadThemeConfigStatus(), loadCurrentTheme()])
  }

  const clearError = (): void => {
    error.value = null
  }

  return {
    configStatus: readonly(configStatus),
    currentTheme: readonly(currentTheme),
    availableThemes: readonly(availableThemes),
    operationState: readonly(operationState),
    error: readonly(error),
    lastOperation: readonly(lastOperation),

    themeConfig,
    currentThemeName,
    isSystemDark,
    isFollowingSystem,
    isLoading,
    themeOptions,

    switchToTheme: themeSwitcher.switchToTheme.bind(themeSwitcher),
    setFollowSystem: themeSwitcher.setFollowSystem.bind(themeSwitcher),
    enableFollowSystem: (lightTheme: string, darkTheme: string) =>
      themeSwitcher.setFollowSystem(true, lightTheme, darkTheme),
    disableFollowSystem: () => themeSwitcher.setFollowSystem(false),

    initialize,
    loadThemeConfigStatus,
    loadCurrentTheme,
    clearError,
  }
})
