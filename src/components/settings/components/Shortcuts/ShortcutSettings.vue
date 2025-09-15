<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.shortcuts.title') }}</h2>

    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('shortcuts.loading') }}</span>
    </div>

    <div v-else class="settings-group">
      <h3 class="settings-group-title">{{ t('shortcuts.title') }}</h3>

      <SettingsCard>
        <div
          v-for="action in allActions"
          :key="action.key"
          class="settings-item shortcut-item"
          :class="{
            'shortcut-item--editing': isEditing(action.key),
            'shortcut-item--configured': action.shortcut,
          }"
          @click="startEdit(action.key)"
          @keydown="handleKeyDown($event, action.key)"
          @blur="stopEdit(action.key)"
          tabindex="0"
        >
          <div class="settings-item-header">
            <div class="settings-label">{{ action.displayName }}</div>
          </div>
          <div class="settings-item-control">
            <span v-if="!isEditing(action.key)" class="shortcut-display">
              <template v-if="action.shortcut">
                <span v-for="modifier in action.shortcut.modifiers" :key="modifier" class="shortcut-modifier">
                  {{ modifier }}
                </span>
                <span class="shortcut-key">{{ action.shortcut.key }}</span>
              </template>
              <span v-else class="shortcut-not-configured">{{ t('shortcuts.not_configured') }}</span>
            </span>
            <span v-else class="shortcut-editing-hint">{{ t('shortcuts.editing_hint') }}</span>
          </div>
        </div>

        <!-- 重置快捷键功能直接放在同一个卡片中 -->
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('shortcuts.reset_shortcuts') }}</div>
            <div class="settings-description">{{ t('shortcuts.reset_description') }}</div>
          </div>
          <div class="settings-item-control">
            <x-popconfirm
              :title="t('shortcuts.reset_confirm_title')"
              :description="t('shortcuts.reset_confirm_message')"
              :confirm-text="t('common.confirm')"
              :cancel-text="t('common.cancel')"
              type="danger"
              placement="top"
              :trigger-text="t('shortcuts.reset_to_default')"
              trigger-button-variant="outline"
              :trigger-button-props="{ disabled: loading }"
              @confirm="handleReset"
            />
          </div>
        </div>
      </SettingsCard>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  import { useShortcuts } from '@/composables/useShortcuts'

  import SettingsCard from '../../SettingsCard.vue'

  import type { ShortcutBinding } from '@/types'

  const {
    config,
    loading,

    initialize,
    addShortcut,
    removeShortcut,
    resetToDefaults,
  } = useShortcuts()

  const editingActionKey = ref<string | null>(null)
  const capturedShortcut = ref<{ key: string; modifiers: string[] } | null>(null)
  const { t } = useI18n()

  const handleReset = async () => {
    await resetToDefaults()

    if ((window as any).reloadShortcuts) {
      await (window as any).reloadShortcuts()
    }
  }

  const allActionKeys = [
    'copy_to_clipboard',
    'paste_from_clipboard',
    'terminal_search',
    'open_settings',
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
    'toggle_ai_sidebar',
    'toggle_window_pin',
  ]

  const findShortcut = (actionKey: string) => {
    if (!config.value) return null

    for (const shortcut of config.value) {
      if (shortcut.action === actionKey) {
        return shortcut
      }
    }
    return null
  }

  const allActions = computed(() => {
    return allActionKeys.map(actionKey => ({
      key: actionKey,
      displayName: t(`shortcuts.actions.${actionKey}`) || actionKey,
      shortcut: findShortcut(actionKey),
    }))
  })

  const isEditing = (actionKey: string) => editingActionKey.value === actionKey

  const startEdit = (actionKey: string) => {
    editingActionKey.value = actionKey
    capturedShortcut.value = null
  }

  const stopEdit = (actionKey: string) => {
    if (editingActionKey.value === actionKey) {
      editingActionKey.value = null
      if (capturedShortcut.value) {
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

    setTimeout(() => stopEdit(actionKey), 100)
  }

  const saveShortcut = async (actionKey: string, shortcut: { key: string; modifiers: string[] }) => {
    const shortcutBinding: ShortcutBinding = {
      key: shortcut.key,
      modifiers: shortcut.modifiers,
      action: actionKey,
    }

    await removeExistingShortcut(actionKey)

    await addShortcut(shortcutBinding)

    if ((window as any).reloadShortcuts) {
      await (window as any).reloadShortcuts()
    }
  }

  const removeExistingShortcut = async (actionKey: string) => {
    if (!config.value) return

    for (let i = 0; i < config.value.length; i++) {
      if (config.value[i].action === actionKey) {
        await removeShortcut(i)
        return
      }
    }
  }

  // 初始化方法，供外部调用
  const init = async () => {
    // 强制重新初始化，确保数据正确加载
    await initialize()
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })
</script>

<style scoped>
  .shortcut-item {
    cursor: pointer;
    transition: all 0.2s ease;
    border-radius: var(--border-radius);
    min-height: 60px;
  }

  .shortcut-item:hover {
    background: transparent;
  }

  .shortcut-item:focus {
    outline: none;
    background: transparent;
  }

  .shortcut-item--editing {
    background: var(--color-primary-alpha);
    animation: pulse 1.5s infinite;
  }

  .shortcut-display {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    justify-content: flex-end;
    min-height: 28px;
  }

  .shortcut-modifier {
    background: var(--bg-600);
    border: none;
    padding: 6px 10px;
    border-radius: var(--border-radius);
    font-size: 12px;
    color: var(--text-200);
    font-weight: 500;
    min-width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.05) inset;
  }

  .shortcut-key {
    background: var(--bg-600);
    border: none;
    color: var(--text-200);
    padding: 6px 10px;
    border-radius: var(--border-radius);
    font-size: 12px;
    font-weight: 500;
    min-width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.05) inset;
  }

  .shortcut-not-configured {
    color: var(--text-400);
    font-style: italic;
    min-height: 28px;
    display: flex;
    align-items: center;
  }

  .shortcut-editing-hint {
    color: var(--color-primary);
    font-style: italic;
    min-height: 28px;
    display: flex;
    align-items: center;
  }

  .settings-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px;
    color: var(--text-300);
  }

  .settings-loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.7;
    }
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 480px) {
    .shortcut-display {
      gap: 1px;
    }

    .shortcut-modifier,
    .shortcut-key {
      padding: 1px 3px;
      font-size: 9px;
    }
  }

  @media (max-width: 320px) {
    .settings-warning {
      padding: 6px 8px;
      font-size: 10px;
    }
  }
</style>
