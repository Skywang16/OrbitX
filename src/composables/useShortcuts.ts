/**
 * 快捷键组合式API
 *
 * 提供便捷的快捷键使用和管理接口
 */

import { computed, ref, watch, onMounted } from 'vue'
import { useShortcutStore } from '@/stores/shortcuts'
import { ShortcutApi } from '@/api/shortcuts'
import {
  ShortcutBinding,
  ShortcutCategory,
  Platform,
  ShortcutSearchOptions,
  ShortcutOperationOptions,
  ShortcutFormatOptions,
} from '@/api/shortcuts/types'

/**
 * 快捷键管理组合式API
 */
export const useShortcuts = () => {
  const store = useShortcutStore()

  // 初始化
  const initialize = async () => {
    if (!store.state.initialized) {
      await store.initialize()
    }
  }

  // 响应式状态
  const config = computed(() => store.state.config)
  const loading = computed(() => store.state.loading)
  const error = computed(() => store.state.error)
  const hasConflicts = computed(() => store.hasConflicts)
  const hasValidationErrors = computed(() => store.hasValidationErrors)
  const statistics = computed(() => store.state.statistics)
  const currentPlatform = computed(() => store.state.currentPlatform)

  // 快捷键操作
  const addShortcut = (category: ShortcutCategory, shortcut: ShortcutBinding, options?: ShortcutOperationOptions) =>
    store.addShortcut(category, shortcut, options)

  const removeShortcut = (category: ShortcutCategory, index: number) => store.removeShortcut(category, index)

  const updateShortcut = (
    category: ShortcutCategory,
    index: number,
    shortcut: ShortcutBinding,
    options?: ShortcutOperationOptions
  ) => store.updateShortcut(category, index, shortcut, options)

  const searchShortcuts = (options: ShortcutSearchOptions) => store.searchShortcuts(options)

  const resetToDefaults = () => store.resetToDefaults()

  const exportConfig = () => store.exportConfig()

  const importConfig = (json: string) => store.importConfig(json)

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
    searchShortcuts,
    resetToDefaults,
    exportConfig,
    importConfig,
    clearError,
  }
}

/**
 * 快捷键格式化组合式API
 */
export const useShortcutFormatter = () => {
  const store = useShortcutStore()

  const formatShortcut = (binding: ShortcutBinding, options?: ShortcutFormatOptions): string => {
    const finalOptions: ShortcutFormatOptions = {
      platform: store.state.currentPlatform || Platform.Linux,
      useSymbols: true,
      separator: '+',
      ...options,
    }

    return ShortcutApi.formatShortcut(binding, finalOptions)
  }

  const formatShortcutList = (bindings: ShortcutBinding[], options?: ShortcutFormatOptions): string[] => {
    return bindings.map(binding => formatShortcut(binding, options))
  }

  return {
    formatShortcut,
    formatShortcutList,
  }
}

/**
 * 快捷键验证组合式API
 */
export const useShortcutValidation = () => {
  const store = useShortcutStore()

  const validateBinding = async (binding: ShortcutBinding) => {
    return await ShortcutApi.validateBinding(binding)
  }

  const validateConfig = async () => {
    return await store.validateCurrentConfig()
  }

  const detectConflicts = async () => {
    return await store.detectCurrentConflicts()
  }

  const lastValidation = computed(() => store.state.lastValidation)
  const lastConflictDetection = computed(() => store.state.lastConflictDetection)

  return {
    validateBinding,
    validateConfig,
    detectConflicts,
    lastValidation,
    lastConflictDetection,
  }
}

/**
 * 快捷键搜索组合式API
 */
export const useShortcutSearch = () => {
  const searchQuery = ref('')
  const searchResults = ref<Awaited<ReturnType<typeof ShortcutApi.searchShortcuts>> | null>(null)
  const searching = ref(false)

  const search = async (options?: Partial<ShortcutSearchOptions>) => {
    if (!searchQuery.value.trim() && !options) return

    searching.value = true
    try {
      const searchOptions: ShortcutSearchOptions = {
        query: searchQuery.value.trim() || undefined,
        ...options,
      }

      searchResults.value = await ShortcutApi.searchShortcuts(searchOptions)
    } catch (error) {
      console.error('搜索快捷键失败:', error)
      searchResults.value = null
    } finally {
      searching.value = false
    }
  }

  const clearSearch = () => {
    searchQuery.value = ''
    searchResults.value = null
  }

  // 自动搜索
  watch(searchQuery, () => {
    if (searchQuery.value.trim()) {
      search()
    } else {
      clearSearch()
    }
  })

  return {
    searchQuery,
    searchResults,
    searching,
    search,
    clearSearch,
  }
}

/**
 * 快捷键监听组合式API
 */
export const useShortcutListener = () => {
  const activeShortcuts = ref<Set<string>>(new Set())
  const pressedKeys = ref<Set<string>>(new Set())
  const pressedModifiers = ref<Set<string>>(new Set())

  const keyMap: Record<string, string> = {
    Control: 'ctrl',
    Meta: 'cmd',
    Alt: 'alt',
    Shift: 'shift',
    ' ': 'Space',
  }

  const normalizeKey = (key: string): string => {
    return keyMap[key] || key.toLowerCase()
  }

  const formatKeyCombo = (): string => {
    const modifiers = Array.from(pressedModifiers.value).sort()
    const keys = Array.from(pressedKeys.value)
    return [...modifiers, ...keys].join('+')
  }

  const handleKeyDown = (event: KeyboardEvent) => {
    const key = normalizeKey(event.key)

    // 处理修饰键
    if (['ctrl', 'cmd', 'alt', 'shift'].includes(key)) {
      pressedModifiers.value.add(key)
    } else {
      pressedKeys.value.add(key)
    }

    // 更新当前激活的快捷键组合
    const combo = formatKeyCombo()
    if (combo) {
      activeShortcuts.value.add(combo)
    }
  }

  const handleKeyUp = (event: KeyboardEvent) => {
    const key = normalizeKey(event.key)

    // 处理修饰键
    if (['ctrl', 'cmd', 'alt', 'shift'].includes(key)) {
      pressedModifiers.value.delete(key)
    } else {
      pressedKeys.value.delete(key)
    }

    // 如果没有按键了，清空激活的快捷键
    if (pressedKeys.value.size === 0 && pressedModifiers.value.size === 0) {
      activeShortcuts.value.clear()
    }
  }

  const startListening = () => {
    document.addEventListener('keydown', handleKeyDown)
    document.addEventListener('keyup', handleKeyUp)
  }

  const stopListening = () => {
    document.removeEventListener('keydown', handleKeyDown)
    document.removeEventListener('keyup', handleKeyUp)
    pressedKeys.value.clear()
    pressedModifiers.value.clear()
    activeShortcuts.value.clear()
  }

  const isShortcutActive = (binding: ShortcutBinding): boolean => {
    const combo = ShortcutApi.formatShortcut(binding, { separator: '+' })
    return activeShortcuts.value.has(combo)
  }

  const getCurrentCombo = computed(() => formatKeyCombo())

  return {
    activeShortcuts: computed(() => Array.from(activeShortcuts.value)),
    pressedKeys: computed(() => Array.from(pressedKeys.value)),
    pressedModifiers: computed(() => Array.from(pressedModifiers.value)),
    currentCombo: getCurrentCombo,
    startListening,
    stopListening,
    isShortcutActive,
  }
}

/**
 * 快捷键编辑器组合式API
 */
export const useShortcutEditor = () => {
  const editingShortcut = ref<ShortcutBinding | null>(null)
  const editingCategory = ref<ShortcutCategory | null>(null)
  const editingIndex = ref<number | null>(null)
  const isEditing = ref(false)

  const startEdit = (shortcut: ShortcutBinding, category: ShortcutCategory, index?: number) => {
    editingShortcut.value = { ...shortcut }
    editingCategory.value = category
    editingIndex.value = index ?? null
    isEditing.value = true
  }

  const cancelEdit = () => {
    editingShortcut.value = null
    editingCategory.value = null
    editingIndex.value = null
    isEditing.value = false
  }

  const saveEdit = async () => {
    if (!editingShortcut.value || !editingCategory.value) {
      throw new Error('没有正在编辑的快捷键')
    }

    const store = useShortcutStore()

    if (editingIndex.value !== null) {
      // 更新现有快捷键
      await store.updateShortcut(editingCategory.value, editingIndex.value, editingShortcut.value)
    } else {
      // 添加新快捷键
      await store.addShortcut(editingCategory.value, editingShortcut.value)
    }

    cancelEdit()
  }

  const updateEditingShortcut = (updates: Partial<ShortcutBinding>) => {
    if (editingShortcut.value) {
      editingShortcut.value = { ...editingShortcut.value, ...updates }
    }
  }

  return {
    editingShortcut,
    editingCategory,
    editingIndex,
    isEditing,
    startEdit,
    cancelEdit,
    saveEdit,
    updateEditingShortcut,
  }
}

/**
 * 自动初始化快捷键系统的组合式API
 */
export const useShortcutsAutoInit = () => {
  const { initialize, error } = useShortcuts()

  onMounted(async () => {
    try {
      await initialize()
    } catch (err) {
      console.error('快捷键系统初始化失败:', err)
    }
  })

  return {
    error,
  }
}
