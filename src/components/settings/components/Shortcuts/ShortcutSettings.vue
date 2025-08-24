<template>
  <div class="shortcut-settings">
    <div class="settings-group">
      <h3 class="section-title">快捷键设置</h3>
      <!-- 冲突警告 -->
      <div v-if="hasConflicts" class="alert alert-warning">
        <span class="alert-icon">⚠️</span>
        <span>检测到 {{ conflictCount }} 个快捷键冲突</span>
      </div>

      <!-- 动作列表 -->
      <div class="actions-list">
        <div v-if="loading" class="loading-state">
          <div class="loading-spinner"></div>
          <span>加载中...</span>
        </div>
        <div v-else>
          <div class="action-category">
            <h4>全局快捷键</h4>
            <div class="action-items">
              <div v-for="action in globalActions" :key="action.key" class="action-item">
                <div class="action-name">{{ action.displayName }}</div>
                <div
                  class="shortcut-key-editor"
                  :class="{ editing: isEditing(action.key), configured: action.shortcut }"
                  @click="startEdit(action.key)"
                  @keydown="handleKeyDown($event, action.key)"
                  @blur="stopEdit(action.key)"
                  tabindex="0"
                >
                  <span v-if="!isEditing(action.key)" class="shortcut-display">
                    <template v-if="action.shortcut">
                      <span v-for="modifier in action.shortcut.modifiers" :key="modifier" class="modifier">
                        {{ modifier }}
                      </span>
                      <span class="key">{{ action.shortcut.key }}</span>
                    </template>
                    <span v-else class="not-configured">点击配置</span>
                  </span>
                  <span v-else class="editing-hint">按下新的快捷键组合...</span>
                </div>
              </div>
            </div>
          </div>

          <div class="action-category">
            <h4>终端快捷键</h4>
            <div class="action-items">
              <div v-for="action in terminalActions" :key="action.key" class="action-item">
                <div class="action-name">{{ action.displayName }}</div>
                <div
                  class="shortcut-key-editor"
                  :class="{ editing: isEditing(action.key), configured: action.shortcut }"
                  @click="startEdit(action.key)"
                  @keydown="handleKeyDown($event, action.key)"
                  @blur="stopEdit(action.key)"
                  tabindex="0"
                >
                  <span v-if="!isEditing(action.key)" class="shortcut-display">
                    <template v-if="action.shortcut">
                      <span v-for="modifier in action.shortcut.modifiers" :key="modifier" class="modifier">
                        {{ modifier }}
                      </span>
                      <span class="key">{{ action.shortcut.key }}</span>
                    </template>
                    <span v-else class="not-configured">点击配置</span>
                  </span>
                  <span v-else class="editing-hint">按下新的快捷键组合...</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 操作按钮 -->
      <div class="actions-section">
        <x-button variant="outline" @click="handleReset" :disabled="loading">重置到默认</x-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { useShortcuts } from '@/composables/useShortcuts'
  import { createMessage } from '@/ui/composables/message-api'
  import { useShortcutStore } from '@/stores/shortcuts'
  import { SHORTCUT_ACTIONS } from '@/shortcuts/constants'
  import { confirmWarning } from '@/ui/composables/confirm-api'

  import type { ShortcutBinding } from '@/types'
  import { ShortcutCategory } from '@/types'

  // 组合式API
  const {
    config,
    loading,
    hasConflicts,

    initialize,
    addShortcut,
    removeShortcut,
    resetToDefaults,
  } = useShortcuts()

  const store = useShortcutStore()
  const lastConflictDetection = computed(() => store.lastConflictDetection)

  // 响应式状态
  const editingActionKey = ref<string | null>(null)
  const capturedShortcut = ref<{ key: string; modifiers: string[] } | null>(null)

  // 计算属性
  const conflicts = computed(() => lastConflictDetection.value?.conflicts || [])
  const conflictCount = computed(() => conflicts.value.length)

  // 方法
  const handleReset = async () => {
    try {
      const shouldReset = await confirmWarning(
        '此操作将重置所有快捷键到默认配置，当前的自定义设置将会丢失。',
        '重置快捷键配置'
      )

      if (shouldReset) {
        await resetToDefaults()

        // 只更新监听器配置，不做其他操作
        if ((window as any).reloadShortcuts) {
          await (window as any).reloadShortcuts()
        }

        createMessage.success('快捷键配置已重置为默认')
      }
    } catch (error) {
      handleErrorWithMessage(error, '重置配置失败')
    }
  }

  // 全局动作定义
  const globalActionKeys = ['copy_to_clipboard', 'paste_from_clipboard', 'terminal_search', 'open_settings']

  // 终端动作定义
  const terminalActionKeys = [
    'new_tab',
    'close_tab',
    'clear_terminal',
    'switch_to_tab_1',
    'switch_to_tab_2',
    'switch_to_tab_3',
    'switch_to_tab_4',
    'switch_to_tab_5',
    'switch_to_last_tab',
    'accept_completion',
    'increase_font_size',
    'decrease_font_size',
  ]

  // 查找快捷键配置
  const findShortcut = (actionKey: string) => {
    if (!config.value) return null

    // 在所有类别中查找
    for (const shortcut of [...config.value.global, ...config.value.terminal, ...config.value.custom]) {
      if (shortcut.action === actionKey) {
        return shortcut
      }
    }
    return null
  }

  // 计算全局动作列表
  const globalActions = computed(() => {
    return globalActionKeys.map(actionKey => ({
      key: actionKey,
      displayName: SHORTCUT_ACTIONS[actionKey as keyof typeof SHORTCUT_ACTIONS] || actionKey,
      shortcut: findShortcut(actionKey),
    }))
  })

  // 计算终端动作列表
  const terminalActions = computed(() => {
    return terminalActionKeys.map(actionKey => ({
      key: actionKey,
      displayName: SHORTCUT_ACTIONS[actionKey as keyof typeof SHORTCUT_ACTIONS] || actionKey,
      shortcut: findShortcut(actionKey),
    }))
  })

  // 编辑状态管理
  const isEditing = (actionKey: string) => editingActionKey.value === actionKey

  const startEdit = (actionKey: string) => {
    editingActionKey.value = actionKey
    capturedShortcut.value = null
  }

  const stopEdit = (actionKey: string) => {
    if (editingActionKey.value === actionKey) {
      editingActionKey.value = null
      if (capturedShortcut.value) {
        // 保存新的快捷键
        saveShortcut(actionKey, capturedShortcut.value)
      }
      capturedShortcut.value = null
    }
  }

  const handleKeyDown = (event: KeyboardEvent, actionKey: string) => {
    if (!isEditing(actionKey)) return

    event.preventDefault()
    event.stopPropagation()

    const modifiers: string[] = []
    if (event.ctrlKey) modifiers.push('ctrl')
    if (event.metaKey) modifiers.push('cmd')
    if (event.altKey) modifiers.push('alt')
    if (event.shiftKey) modifiers.push('shift')

    let key = event.key
    if (key === ' ') key = 'Space'
    if (key === 'Control' || key === 'Meta' || key === 'Alt' || key === 'Shift') return

    capturedShortcut.value = { key, modifiers }

    // 自动保存并退出编辑模式
    setTimeout(() => stopEdit(actionKey), 100)
  }

  const saveShortcut = async (actionKey: string, shortcut: { key: string; modifiers: string[] }) => {
    try {
      const shortcutBinding: ShortcutBinding = {
        key: shortcut.key,
        modifiers: shortcut.modifiers,
        action: actionKey,
      }

      // 确定类别
      const category = globalActionKeys.includes(actionKey) ? ShortcutCategory.Global : ShortcutCategory.Terminal

      // 先删除现有的配置（内部处理）
      await removeExistingShortcut(actionKey)

      // 添加新配置
      await addShortcut(category, shortcutBinding)

      // 重新加载快捷键监听器配置（仅更新监听器，不刷新页面）
      if ((window as any).reloadShortcuts) {
        await (window as any).reloadShortcuts()
      }

      createMessage.success(
        `${SHORTCUT_ACTIONS[actionKey as keyof typeof SHORTCUT_ACTIONS] || actionKey} 快捷键设置成功`
      )
    } catch (error) {
      handleErrorWithMessage(error, '保存快捷键失败')
    }
  }

  const removeExistingShortcut = async (actionKey: string) => {
    if (!config.value) return

    // 在所有类别中查找并删除现有配置（静默处理）
    const categories = [
      { name: ShortcutCategory.Global, shortcuts: config.value.global },
      { name: ShortcutCategory.Terminal, shortcuts: config.value.terminal },
      { name: ShortcutCategory.Custom, shortcuts: config.value.custom },
    ]

    for (const cat of categories) {
      const index = cat.shortcuts.findIndex(s => s.action === actionKey)
      if (index !== -1) {
        await removeShortcut(cat.name, index)
        return
      }
    }
  }

  // 生命周期
  onMounted(async () => {
    // 使用新的初始化检查机制
    if (!store.initialized && !loading.value) {
      try {
        await initialize()
      } catch (err) {
        handleErrorWithMessage(err, '快捷键设置初始化失败')
      }
    }
  })
</script>

<style scoped>
  .shortcut-settings {
    padding: 24px 28px;
    background: var(--bg-200);
    margin-bottom: 24px;
  }

  .settings-group {
    margin-bottom: 32px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-300);
  }

  .settings-group:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  .section-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 16px 0;
    padding: 0;
  }

  .alert {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 4px;
    margin-bottom: 16px;
    background: var(--color-warning);
    color: white;
    font-size: 14px;
  }

  .actions-list {
    margin-bottom: 24px;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px;
    color: var(--text-400);
    background: var(--bg-300);
    border-radius: 4px;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .action-category {
    margin-bottom: 24px;
  }

  .action-category h4 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 500;
    color: var(--text-200);
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border-300);
  }

  .action-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .action-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--bg-300);
    border-radius: 4px;
    gap: 20px;
  }

  .action-item:hover {
    background: var(--bg-400);
  }

  .action-name {
    flex: 1;
    color: var(--text-200);
    font-size: 15px;
  }

  .shortcut-key-editor {
    flex: 2;
    min-width: 220px;
    padding: 10px 14px;
    background: var(--bg-400);
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .shortcut-key-editor:hover {
    background: var(--bg-500);
  }

  .shortcut-key-editor.editing {
    border-color: var(--color-primary);
    background: var(--color-primary-alpha);
    box-shadow: 0 0 0 1px var(--color-primary);
  }

  .shortcut-display {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .not-configured {
    color: var(--text-400);
    font-size: 14px;
  }

  .modifier,
  .key {
    padding: 4px 8px;
    background: var(--bg-500);
    border-radius: 3px;
    font-size: 12px;
    font-family: var(--font-family-mono);
    color: var(--text-200);
  }

  .key {
    background: var(--color-primary);
    color: white;
  }

  .actions-section {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding-top: 16px;
  }

  .actions-section :deep(.x-button) {
    background: transparent;
    border: 1px solid var(--border-300);
    color: var(--text-300);
    border-radius: 4px;
    padding: 8px 16px;
    font-size: 14px;
  }

  .actions-section :deep(.x-button:hover) {
    background: var(--bg-400);
    color: var(--text-200);
    border-color: var(--border-400);
  }

  .actions-section :deep(.x-button:disabled) {
    opacity: 0.5;
  }
</style>
