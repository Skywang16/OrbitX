/**
 * 主题状态管理 Store
 * 使用 Pinia 实现集中化的主题状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { themeAPI } from '@/api/config'
import type { ThemeConfigStatus, Theme, ThemeInfo } from '@/types/domain/theme'
import { applyThemeToUI } from '@/utils/themeApplier'

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
  const availableThemes = ref<ThemeInfo[]>([])

  const operationState = ref<ThemeOperationState>(ThemeOperationState.IDLE)
  const error = ref<string | null>(null)
  const lastOperation = ref<ThemeOperation | null>(null)

  const themeConfig = computed(() => configStatus.value?.themeConfig || null)
  const currentThemeName = computed(() => configStatus.value?.currentThemeName || '')
  const isSystemDark = computed(() => configStatus.value?.isSystemDark)
  const isFollowingSystem = computed(() => themeConfig.value?.followSystem || false)
  const isLoading = computed(() => operationState.value !== ThemeOperationState.IDLE)

  const themeOptions = computed(() => {
    return availableThemes.value.map(theme => ({
      value: theme.name,
      label: theme.name,
      type: theme.themeType,
      isCurrent: theme.isCurrent,
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

    async switchToTheme(themeName: string): Promise<void> {
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

        // 5. 确保 availableThemes 状态与真实主题同步（如果主题名称不同）
        if (newTheme.name !== themeName) {
          this.syncAvailableThemesState(newTheme.name)
        }
      })
    }

    async setFollowSystem(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
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

        // 5. 确保 availableThemes 状态与真实主题同步
        this.syncAvailableThemesState(newTheme.name)
      })
    }

    private async executeOperation(operation: ThemeOperation, action: () => Promise<void>): Promise<void> {
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

    private optimisticUpdateTheme(themeName: string): void {
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

      // 立即更新 availableThemes 中的 isCurrent 状态以提供即时反馈
      this.syncAvailableThemesState(themeName)
    }

    private optimisticUpdateFollowSystem(followSystem: boolean, lightTheme?: string, darkTheme?: string): void {
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

    private createStateSnapshot(): StateSnapshot {
      return {
        configStatus: this.store.configStatus.value,
        currentTheme: this.store.currentTheme.value,
      }
    }

    private restoreStateSnapshot(snapshot: StateSnapshot): void {
      this.store.configStatus.value = snapshot.configStatus
      this.store.currentTheme.value = snapshot.currentTheme

      // 如果有当前主题，需要重新应用到UI以回滚视觉效果
      if (snapshot.currentTheme) {
        applyThemeToUI(snapshot.currentTheme)
        // 同时回滚 availableThemes 状态
        this.syncAvailableThemesState(snapshot.currentTheme.name)
      }
    }

    private syncAvailableThemesState(currentThemeName: string): void {
      // 只有在状态真正需要改变时才更新，避免不必要的响应式更新
      const needsUpdate = availableThemes.value.some(
        theme =>
          (theme.name === currentThemeName && !theme.isCurrent) || (theme.name !== currentThemeName && theme.isCurrent)
      )

      if (needsUpdate) {
        availableThemes.value = availableThemes.value.map(theme => ({
          ...theme,
          isCurrent: theme.name === currentThemeName,
        }))
      }
    }
  }

  const themeSwitcher = new ThemeSwitcher()

  // 数据加载

  async function loadThemeConfigStatus(): Promise<void> {
    try {
      operationState.value = ThemeOperationState.UPDATING_CONFIG
      const status = await themeAPI.getThemeConfigStatus()
      configStatus.value = status
      availableThemes.value = status.availableThemes
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      operationState.value = ThemeOperationState.IDLE
    }
  }

  async function loadCurrentTheme(): Promise<void> {
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

  async function initialize(): Promise<void> {
    await Promise.all([loadThemeConfigStatus(), loadCurrentTheme()])
  }

  function clearError(): void {
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
