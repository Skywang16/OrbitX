/**
 * 快捷键状态管理 Store
 *
 * 使用 Pinia 管理快捷键配置的响应式状态和操作方法
 */

import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { shortcutsApi } from '@/api'
import type {
  ShortcutsConfig,
  ShortcutBinding,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
} from '@/types'

export const useShortcutStore = defineStore('shortcuts', () => {
  // 简化状态：扁平化结构
  const config = ref<ShortcutsConfig | null>(null)
  const currentPlatform = ref<Platform | null>(null)
  const statistics = ref<ShortcutStatistics | null>(null)
  const lastValidation = ref<ShortcutValidationResult | null>(null)
  const lastConflictDetection = ref<ConflictDetectionResult | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const initialized = ref(false)

  // 简化计算属性
  const hasConfig = computed(() => config.value !== null)
  const hasConflicts = computed(() => lastConflictDetection.value?.has_conflicts ?? false)
  const hasValidationErrors = computed(() => lastValidation.value && !lastValidation.value.is_valid)
  const totalShortcuts = computed(() => statistics.value?.total_count ?? 0)

  // 操作方法

  // 通用异步操作处理
  const withLoading = async <T>(operation: () => Promise<T>): Promise<T> => {
    loading.value = true
    error.value = null
    try {
      return await operation()
    } catch (err) {
      error.value = `操作失败: ${err}`
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 初始化快捷键 Store
   */
  const initialize = async (): Promise<void> => {
    if (initialized.value) return

    return withLoading(async () => {
      // 并行加载配置、平台信息和统计信息
      const [configData, platform, stats] = await Promise.all([
        shortcutsApi.getConfig(),
        shortcutsApi.getCurrentPlatform(),
        shortcutsApi.getStatistics(),
      ])

      config.value = configData
      currentPlatform.value = platform
      statistics.value = stats
      initialized.value = true

      // 初始化时进行一次验证和冲突检测
      await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
    })
  }

  /**
   * 刷新配置
   */
  const refreshConfig = async (): Promise<void> => {
    return withLoading(async () => {
      const [configData, stats] = await Promise.all([shortcutsApi.getConfig(), shortcutsApi.getStatistics()])

      config.value = configData
      statistics.value = stats

      // 刷新后重新验证
      await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
    })
  }

  /**
   * 验证当前配置
   */
  const validateCurrentConfig = async (): Promise<ShortcutValidationResult> => {
    if (!config.value) {
      throw new Error('没有可验证的配置')
    }

    const result = await shortcutsApi.validateConfig(config.value)
    lastValidation.value = result
    return result
  }

  /**
   * 检测当前配置的冲突
   */
  const detectCurrentConflicts = async (): Promise<ConflictDetectionResult> => {
    if (!config.value) {
      throw new Error('没有可检测的配置')
    }

    const result = await shortcutsApi.detectConflicts(config.value)
    lastConflictDetection.value = result
    return result
  }

  /**
   * 添加快捷键
   */
  const addShortcut = async (shortcut: ShortcutBinding): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.addShortcut(shortcut)
      await refreshConfig()
    })
  }

  /**
   * 删除快捷键
   */
  const removeShortcut = async (index: number): Promise<ShortcutBinding> => {
    return withLoading(async () => {
      const removedShortcut = await shortcutsApi.removeShortcut(index)
      await refreshConfig()
      return removedShortcut
    })
  }

  /**
   * 更新快捷键
   */
  const updateShortcut = async (index: number, shortcut: ShortcutBinding): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.updateShortcut(index, shortcut)
      await refreshConfig()
    })
  }

  /**
   * 重置到默认配置
   */
  const resetToDefaults = async (): Promise<void> => {
    return withLoading(async () => {
      await shortcutsApi.resetToDefaults()
      await refreshConfig()
    })
  }

  // 监听配置变化，自动重新验证
  watch(
    () => config.value,
    async newConfig => {
      if (newConfig && initialized.value) {
        try {
          await Promise.all([validateCurrentConfig(), detectCurrentConflicts()])
        } catch (err) {
          console.error('快捷键配置验证失败:', err)
        }
      }
    },
    { deep: true }
  )

  return {
    config,
    currentPlatform,
    statistics,
    lastValidation,
    lastConflictDetection,
    loading,
    error,
    initialized,

    hasConfig,
    hasConflicts,
    hasValidationErrors,
    totalShortcuts,

    // 操作方法
    initialize,
    refreshConfig,
    validateCurrentConfig,
    detectCurrentConflicts,
    addShortcut,
    removeShortcut,
    updateShortcut,
    resetToDefaults,
    clearError: () => {
      error.value = null
    },
  }
})
