<template>
  <div class="shortcut-settings">
    <!-- Loading State -->
    <div v-if="loading" class="settings-loading">
      <div class="settings-loading-spinner"></div>
      <span>{{ t('shortcuts.loading') }}</span>
    </div>

    <template v-else>
      <!-- Shortcuts List -->
      <div class="settings-section">
        <h3 class="settings-section-title">{{ t('shortcuts.keyboard_shortcuts') || 'Keyboard Shortcuts' }}</h3>

        <SettingsCard>
          <div
            v-for="action in allActions"
            :key="action.key"
            class="settings-item shortcut-item"
            :class="{
              'is-editing': isEditing(action.key),
              'is-configured': action.shortcut,
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
              <template v-if="!isEditing(action.key)">
                <div v-if="action.shortcut" class="shortcut-keys">
                  <kbd v-for="modifier in action.shortcut.modifiers" :key="modifier" class="key-cap modifier">
                    {{ formatModifier(modifier) }}
                  </kbd>
                  <kbd class="key-cap">{{ formatKey(action.shortcut.key) }}</kbd>
                </div>
                <span v-else class="shortcut-empty">{{ t('shortcuts.click_to_set') || 'Click to set' }}</span>
              </template>
              <span v-else class="shortcut-recording">
                <span class="recording-dot"></span>
                {{ t('shortcuts.press_keys') || 'Press keys...' }}
              </span>
            </div>
          </div>
        </SettingsCard>
      </div>

      <!-- Reset Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">{{ t('shortcuts.reset_title') || 'Reset' }}</h3>

        <SettingsCard>
          <div class="settings-item">
            <div class="settings-item-header">
              <div class="settings-label">{{ t('shortcuts.reset_shortcuts') }}</div>
              <div class="settings-description">{{ t('shortcuts.reset_description') }}</div>
            </div>
            <div class="settings-item-control">
              <x-button variant="secondary" size="small" :disabled="loading" @click="confirmReset">
                {{ t('shortcuts.reset_to_default') }}
              </x-button>
            </div>
          </div>
        </SettingsCard>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { confirm } from '@tauri-apps/plugin-dialog'

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

  // 组件挂载时自动初始化
  onMounted(async () => {
    await initialize()
  })

  const editingActionKey = ref<string | null>(null)
  const capturedShortcut = ref<{ key: string; modifiers: string[] } | null>(null)
  const { t } = useI18n()

  const handleReset = async () => {
    await resetToDefaults()

    await (window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts?.()
  }

  const confirmReset = async () => {
    const confirmed = await confirm(t('shortcuts.reset_confirm_message'), {
      title: t('shortcuts.reset_confirm_title'),
      kind: 'warning',
    })
    if (confirmed) {
      handleReset()
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

    await (window as typeof window & { reloadShortcuts?: () => void }).reloadShortcuts?.()
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

  // Format modifier for display
  const formatModifier = (modifier: string) => {
    const modifierMap: Record<string, string> = {
      cmd: '⌘',
      ctrl: '⌃',
      alt: '⌥',
      shift: '⇧',
    }
    return modifierMap[modifier] || modifier
  }

  // Format key for display
  const formatKey = (key: string) => {
    const keyMap: Record<string, string> = {
      ArrowUp: '↑',
      ArrowDown: '↓',
      ArrowLeft: '←',
      ArrowRight: '→',
      Enter: '↵',
      Escape: 'Esc',
      Backspace: '⌫',
      Delete: '⌦',
      Tab: '⇥',
      Space: '␣',
    }
    return keyMap[key] || key.toUpperCase()
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
  .shortcut-settings {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Shortcut Item */
  .shortcut-item {
    cursor: pointer;
    transition: background-color 0.15s ease;
    min-height: 56px;
  }

  .shortcut-item:hover {
    background: var(--bg-250, color-mix(in srgb, var(--bg-300) 50%, var(--bg-200)));
  }

  .shortcut-item:focus {
    outline: none;
    background: var(--bg-300);
  }

  .shortcut-item.is-editing {
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  }

  /* Shortcut Keys Display */
  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .key-cap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 8px;
    background: var(--bg-400);
    border-radius: 5px;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-200);
    box-shadow:
      0 1px 0 1px var(--bg-500),
      inset 0 1px 0 0 color-mix(in srgb, white 8%, transparent);
    text-transform: uppercase;
  }

  .key-cap.modifier {
    font-size: 14px;
    min-width: 22px;
    padding: 0 6px;
  }

  /* Empty State */
  .shortcut-empty {
    font-size: 12px;
    color: var(--text-500);
    font-style: italic;
  }

  /* Recording State */
  .shortcut-recording {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-primary);
  }

  .recording-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: recording-pulse 1s ease-in-out infinite;
  }

  @keyframes recording-pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(0.8);
    }
  }
</style>
