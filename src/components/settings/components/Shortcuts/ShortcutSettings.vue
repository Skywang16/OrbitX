<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.shortcuts.title') }}</h2>

    <div v-if="hasConflicts" class="settings-warning">
      <span class="settings-warning-icon">⚠️</span>
      <span>{{ t('shortcuts.conflicts', { count: conflictCount }) }}</span>
    </div>

    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('shortcuts.loading') }}</span>
    </div>
    <div v-else class="settings-group">
      <h3 class="settings-group-title">{{ t('shortcuts.title') }}</h3>
      <div v-for="action in allActions" :key="action.key" class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ action.displayName }}</div>
        </div>
        <div class="settings-item-control">
          <div
            class="shortcut-editor"
            :class="{
              'shortcut-editor--editing': isEditing(action.key),
              'shortcut-editor--configured': action.shortcut,
            }"
            @click="startEdit(action.key)"
            @keydown="handleKeyDown($event, action.key)"
            @blur="stopEdit(action.key)"
            tabindex="0"
          >
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
      </div>
    </div>

    <div v-if="!loading" class="settings-group">
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('shortcuts.reset_shortcuts') }}</div>
          <div class="settings-description">{{ t('shortcuts.reset_description') }}</div>
        </div>
        <div class="settings-item-control">
          <x-button variant="outline" @click="handleReset" :disabled="loading">
            {{ t('shortcuts.reset_to_default') }}
          </x-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { useShortcuts } from '@/composables/useShortcuts'
  import { createMessage } from '@/ui/composables/message-api'
  import { useShortcutStore } from '@/stores/shortcuts'

  import { confirmWarning } from '@/ui/composables/confirm-api'

  import type { ShortcutBinding } from '@/types'

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

  const editingActionKey = ref<string | null>(null)
  const capturedShortcut = ref<{ key: string; modifiers: string[] } | null>(null)
  const { t } = useI18n()

  const conflicts = computed(() => lastConflictDetection.value?.conflicts || [])
  const conflictCount = computed(() => conflicts.value.length)

  const handleReset = async () => {
    try {
      const shouldReset = await confirmWarning(t('shortcuts.reset_confirm_message'), t('shortcuts.reset_confirm_title'))

      if (shouldReset) {
        await resetToDefaults()

        if ((window as any).reloadShortcuts) {
          await (window as any).reloadShortcuts()
        }

        createMessage.success(t('shortcuts.reset_success'))
      }
    } catch (error) {
      handleErrorWithMessage(error, t('shortcuts.reset_failed'))
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
    try {
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

      createMessage.success(
        t('shortcuts.save_success', {
          action: t(`shortcuts.actions.${actionKey}`) || actionKey,
        })
      )
    } catch (error) {
      handleErrorWithMessage(error, t('shortcuts.save_failed'))
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

  onMounted(async () => {
    if (!store.initialized && !loading.value) {
      try {
        await initialize()
      } catch (err) {
        handleErrorWithMessage(err, t('shortcuts.init_failed'))
      }
    }
  })
</script>

<style scoped>
  .shortcut-editor {
    min-width: 100px;
    width: 100%;
    max-width: 200px;
    padding: 6px 12px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    color: var(--text-200);
    font-size: 12px;
    font-family: var(--font-family-mono);
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .shortcut-editor:hover {
    border-color: var(--border-400);
    background: var(--bg-600);
  }

  .shortcut-editor:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .shortcut-editor--editing {
    border-color: var(--color-primary);
    background: var(--color-primary-alpha);
    animation: pulse 1.5s infinite;
  }

  .shortcut-editor--configured {
    background: var(--bg-400);
  }

  .shortcut-display {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .shortcut-modifier {
    background: var(--bg-600);
    padding: 2px 4px;
    border-radius: 2px;
    font-size: 10px;
    color: var(--text-300);
  }

  .shortcut-key {
    background: var(--color-primary);
    color: var(--color-primary-text);
    padding: 2px 6px;
    border-radius: 2px;
    font-size: 10px;
    font-weight: 500;
  }

  .shortcut-not-configured {
    color: var(--text-400);
    font-style: italic;
  }

  .shortcut-editing-hint {
    color: var(--color-primary);
    font-style: italic;
  }

  .settings-warning {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--color-warning-alpha);
    border: 1px solid var(--color-warning);
    border-radius: var(--border-radius);
    color: var(--color-warning-text);
    margin-bottom: 16px;
    font-size: 13px;
  }

  .settings-warning-icon {
    font-size: 16px;
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

  /* 响应式设计 */
  @media (max-width: 480px) {
    .shortcut-editor {
      min-width: 100px;
      padding: 8px 10px;
      font-size: 11px;
    }

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
    .shortcut-editor {
      min-width: 60px;
      padding: 4px 6px;
      font-size: 9px;
    }

    .settings-warning {
      padding: 6px 8px;
      font-size: 10px;
    }
  }
</style>
