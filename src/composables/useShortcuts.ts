/**
 * 快捷键组合式API
 *
 * 提供便捷的快捷键使用和管理接口
 */

import { computed } from 'vue'
import { useShortcutStore } from '@/stores/shortcuts'
import type { ShortcutBinding } from '@/api'

/**
 * 快捷键管理组合式API
 */
export const useShortcuts = () => {
  const store = useShortcutStore()

  // 初始化
  const initialize = async () => {
    // 如果没有初始化过，或者 config 为空，都需要加载数据
    if (!store.initialized || !store.config) {
      await store.refreshConfig()
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
  const addShortcut = (shortcut: ShortcutBinding) => store.addShortcut(shortcut)

  const removeShortcut = (index: number) => store.removeShortcut(index)

  const updateShortcut = (index: number, shortcut: ShortcutBinding) => store.updateShortcut(index, shortcut)

  const resetToDefaults = () => store.resetToDefaults()

  const clearError = () => store.clearError()

  return {
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
