<template>
  <div class="shortcut-settings">
    <!-- 标题和统计信息 -->
    <div class="settings-header">
      <h2 class="settings-title">快捷键设置</h2>
      <ShortcutStatistics />
    </div>

    <!-- 搜索和过滤 -->
    <div class="settings-toolbar">
      <ShortcutSearchBar v-model:filter="searchFilter" @search="handleSearch" @clear="handleClearSearch" />

      <div class="toolbar-actions">
        <button class="btn btn-primary" @click="handleAddShortcut" :disabled="loading">
          <i class="icon-plus"></i>
          添加快捷键
        </button>

        <div class="dropdown">
          <button class="btn btn-secondary dropdown-toggle" :disabled="loading">
            <i class="icon-more"></i>
            更多操作
          </button>
          <div class="dropdown-menu">
            <button @click="handleExport" :disabled="loading">
              <i class="icon-export"></i>
              导出配置
            </button>
            <button @click="handleImport" :disabled="loading">
              <i class="icon-import"></i>
              导入配置
            </button>
            <div class="dropdown-divider"></div>
            <button @click="handleReset" :disabled="loading" class="text-danger">
              <i class="icon-reset"></i>
              重置到默认
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="alert alert-danger">
      <i class="icon-error"></i>
      {{ error }}
      <button class="btn-close" @click="clearError"></button>
    </div>

    <!-- 冲突警告 -->
    <div v-if="hasConflicts" class="alert alert-warning">
      <i class="icon-warning"></i>
      检测到 {{ conflictCount }} 个快捷键冲突，请及时处理
      <button class="btn btn-sm btn-warning" @click="showConflictDialog = true">查看详情</button>
    </div>

    <!-- 快捷键列表 -->
    <div class="settings-content">
      <ShortcutList :items="filteredShortcuts" :loading="loading" @action="handleShortcutAction" />
    </div>

    <!-- 快捷键编辑器对话框 -->
    <ShortcutEditor v-if="showEditor" :options="editorOptions" @save="handleSaveShortcut" @cancel="handleCancelEdit" />

    <!-- 冲突详情对话框 -->
    <ShortcutConflictDialog
      v-if="showConflictDialog"
      :conflicts="conflicts"
      @resolve="handleResolveConflict"
      @close="showConflictDialog = false"
    />

    <!-- 导入文件输入 -->
    <input ref="importFileInput" type="file" accept=".json" style="display: none" @change="handleImportFile" />
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted, watch } from 'vue'
  import { useShortcuts, useShortcutValidation } from '@/composables/useShortcuts'
  import { ShortcutList, ShortcutEditor, ShortcutConflictDialog, ShortcutSearchBar, ShortcutStatistics } from './index'
  import type {
    ShortcutListItem,
    ShortcutSearchFilter,
    ShortcutEditorOptions,
    ShortcutActionEvent,
    ShortcutEditorMode,
    ShortcutActionType,
  } from './types'
  import type { ShortcutBinding, ShortcutCategory } from '@/api/shortcuts/types'

  // 组合式API
  const {
    config,
    loading,
    error,
    hasConflicts,
    statistics,
    initialize,
    addShortcut,
    removeShortcut,
    updateShortcut,
    resetToDefaults,
    exportConfig,
    importConfig,
    clearError,
  } = useShortcuts()

  const { lastConflictDetection } = useShortcutValidation()

  // 响应式状态
  const searchFilter = ref<ShortcutSearchFilter>({
    query: '',
    categories: [],
    conflictsOnly: false,
    errorsOnly: false,
  })

  const showEditor = ref(false)
  const showConflictDialog = ref(false)
  const editorOptions = ref<ShortcutEditorOptions>({
    mode: ShortcutEditorMode.Add,
  })

  const importFileInput = ref<HTMLInputElement>()

  // 计算属性
  const conflicts = computed(() => lastConflictDetection.value?.conflicts || [])
  const conflictCount = computed(() => conflicts.value.length)

  const allShortcuts = computed((): ShortcutListItem[] => {
    if (!config.value) return []

    const items: ShortcutListItem[] = []

    // 添加全局快捷键
    config.value.global.forEach((binding, index) => {
      items.push({
        binding,
        category: 'Global' as ShortcutCategory,
        index,
        hasConflict: hasShortcutConflict(binding, 'Global', index),
      })
    })

    // 添加终端快捷键
    config.value.terminal.forEach((binding, index) => {
      items.push({
        binding,
        category: 'Terminal' as ShortcutCategory,
        index,
        hasConflict: hasShortcutConflict(binding, 'Terminal', index),
      })
    })

    // 添加自定义快捷键
    config.value.custom.forEach((binding, index) => {
      items.push({
        binding,
        category: 'Custom' as ShortcutCategory,
        index,
        hasConflict: hasShortcutConflict(binding, 'Custom', index),
      })
    })

    return items
  })

  const filteredShortcuts = computed(() => {
    let items = allShortcuts.value

    // 按查询关键词过滤
    if (searchFilter.value.query) {
      const query = searchFilter.value.query.toLowerCase()
      items = items.filter(item => {
        const keyMatches = item.binding.key.toLowerCase().includes(query)
        const modifierMatches = item.binding.modifiers.some(m => m.toLowerCase().includes(query))
        const actionMatches =
          typeof item.binding.action === 'string'
            ? item.binding.action.toLowerCase().includes(query)
            : item.binding.action.action_type.toLowerCase().includes(query)

        return keyMatches || modifierMatches || actionMatches
      })
    }

    // 按类别过滤
    if (searchFilter.value.categories.length > 0) {
      items = items.filter(item => searchFilter.value.categories.includes(item.category))
    }

    // 只显示有冲突的
    if (searchFilter.value.conflictsOnly) {
      items = items.filter(item => item.hasConflict)
    }

    // 只显示有错误的
    if (searchFilter.value.errorsOnly) {
      items = items.filter(item => item.hasError)
    }

    return items
  })

  // 方法
  const hasShortcutConflict = (binding: ShortcutBinding, category: ShortcutCategory, index: number): boolean => {
    if (!lastConflictDetection.value) return false

    return lastConflictDetection.value.conflicts.some(conflict =>
      conflict.conflicting_shortcuts.some(
        cs =>
          cs.category === category &&
          cs.binding.key === binding.key &&
          JSON.stringify(cs.binding.modifiers) === JSON.stringify(binding.modifiers)
      )
    )
  }

  const handleSearch = () => {
    // 搜索逻辑已在计算属性中处理
  }

  const handleClearSearch = () => {
    searchFilter.value = {
      query: '',
      categories: [],
      conflictsOnly: false,
      errorsOnly: false,
    }
  }

  const handleAddShortcut = () => {
    editorOptions.value = {
      mode: ShortcutEditorMode.Add,
    }
    showEditor.value = true
  }

  const handleShortcutAction = (event: ShortcutActionEvent) => {
    switch (event.type) {
      case ShortcutActionType.Edit:
        if (event.item) {
          editorOptions.value = {
            mode: ShortcutEditorMode.Edit,
            initialShortcut: event.item.binding,
            initialCategory: event.item.category,
            initialIndex: event.item.index,
          }
          showEditor.value = true
        }
        break

      case ShortcutActionType.Delete:
        if (event.item) {
          handleDeleteShortcut(event.item)
        }
        break

      case ShortcutActionType.Duplicate:
        if (event.item) {
          handleDuplicateShortcut(event.item)
        }
        break
    }
  }

  const handleSaveShortcut = async (shortcut: ShortcutBinding, category: ShortcutCategory, index?: number) => {
    try {
      if (index !== undefined) {
        await updateShortcut(category, index, shortcut)
      } else {
        await addShortcut(category, shortcut)
      }
      showEditor.value = false
    } catch (error) {
      console.error('保存快捷键失败:', error)
    }
  }

  const handleCancelEdit = () => {
    showEditor.value = false
  }

  const handleDeleteShortcut = async (item: ShortcutListItem) => {
    if (confirm('确定要删除这个快捷键吗？')) {
      try {
        await removeShortcut(item.category, item.index)
      } catch (error) {
        console.error('删除快捷键失败:', error)
      }
    }
  }

  const handleDuplicateShortcut = (item: ShortcutListItem) => {
    editorOptions.value = {
      mode: ShortcutEditorMode.Add,
      initialShortcut: { ...item.binding },
      initialCategory: item.category,
    }
    showEditor.value = true
  }

  const handleExport = async () => {
    try {
      const configJson = await exportConfig()
      const blob = new Blob([configJson], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'shortcuts-config.json'
      a.click()
      URL.revokeObjectURL(url)
    } catch (error) {
      console.error('导出配置失败:', error)
    }
  }

  const handleImport = () => {
    importFileInput.value?.click()
  }

  const handleImportFile = async (event: Event) => {
    const target = event.target as HTMLInputElement
    const file = target.files?.[0]

    if (file) {
      try {
        const text = await file.text()
        await importConfig(text)
      } catch (error) {
        console.error('导入配置失败:', error)
      }
    }

    // 清空文件输入
    target.value = ''
  }

  const handleReset = async () => {
    if (confirm('确定要重置所有快捷键到默认配置吗？此操作不可撤销。')) {
      try {
        await resetToDefaults()
      } catch (error) {
        console.error('重置配置失败:', error)
      }
    }
  }

  const handleResolveConflict = (conflictIndex: number) => {
    // 处理冲突解决逻辑
    showConflictDialog.value = false
  }

  // 生命周期
  onMounted(async () => {
    try {
      await initialize()
    } catch (err) {
      console.error('快捷键设置初始化失败:', err)
    }
  })
</script>

<style scoped>
  .shortcut-settings {
    padding: 20px;
    max-width: 1200px;
    margin: 0 auto;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .settings-title {
    font-size: 24px;
    font-weight: 600;
    margin: 0;
  }

  .settings-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    gap: 16px;
  }

  .toolbar-actions {
    display: flex;
    gap: 12px;
  }

  .settings-content {
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 20px;
  }

  .alert {
    padding: 12px 16px;
    border-radius: 6px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .alert-danger {
    background: var(--error-bg);
    color: var(--error-text);
    border: 1px solid var(--error-border);
  }

  .alert-warning {
    background: var(--warning-bg);
    color: var(--warning-text);
    border: 1px solid var(--warning-border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    font-size: 14px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-hover);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn-warning {
    background: var(--warning);
    color: white;
  }

  .btn-close {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    margin-left: auto;
  }

  .dropdown {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow-lg);
    min-width: 160px;
    z-index: 1000;
    display: none;
  }

  .dropdown:hover .dropdown-menu {
    display: block;
  }

  .dropdown-menu button {
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dropdown-menu button:hover {
    background: var(--bg-hover);
  }

  .dropdown-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .text-danger {
    color: var(--error-text);
  }
</style>
