/**
 * 快捷键组合式API
 *
 * 提供便捷的快捷键使用和管理接口
 */

import { computed } from 'vue'
import { useShortcutStore } from '@/stores/shortcuts'
import type { ShortcutBinding, ShortcutCategory } from '@/api'

/**
 * 快捷键管理组合式API
 */
export const useShortcuts = () => {
  const store = useShortcutStore()

  // 初始化
  const initialize = async () => {
    if (!store.initialized) {
      await store.initialize()
    }
  }

  // 响应式状态
  const config = computed(() => store.config)
  const loading = computed(() => store.loading)
  const error = computed(() => store.error)
  const hasConflicts = computed(() => store.hasConflicts)
  const hasValidationErrors = computed(() => store.hasValidationErrors)
  const statistics = computed(() => store.statistics)
  const currentPlatform = computed(() => store.currentPlatform)

  // 快捷键操作
  const addShortcut = (category: ShortcutCategory, shortcut: ShortcutBinding) => store.addShortcut(category, shortcut)

  const removeShortcut = (category: ShortcutCategory, index: number) => store.removeShortcut(category, index)

  const updateShortcut = (category: ShortcutCategory, index: number, shortcut: ShortcutBinding) =>
    store.updateShortcut(category, index, shortcut)

  const resetToDefaults = () => store.resetToDefaults()

  const clearError = () => store.clearError()

  return {
    // 状态
    config,
    loading,
    error,
    hasConflicts,
    hasValidationErrors,
    statistics,
    currentPlatform,

    // 方法
    initialize,
    addShortcut,
    removeShortcut,
    updateShortcut,
    resetToDefaults,
    clearError,
  }
}
