<script setup lang="ts">
  import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
  import { useI18n } from 'vue-i18n'

  export interface CommandItem {
    id: string
    label: string
    description?: string
    icon?: string
    shortcut?: string
    category?: string
    action?: () => void | Promise<void>
    children?: CommandItem[]
  }

  interface Props {
    commands?: CommandItem[]
    placeholder?: string
  }

  interface Emits {
    (e: 'select', command: CommandItem): void
    (e: 'close'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    commands: () => [],
    placeholder: '',
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const visible = ref(false)
  const query = ref('')
  const selectedIndex = ref(0)
  const inputRef = ref<HTMLInputElement>()
  const listRef = ref<HTMLDivElement>()
  const commandStack = ref<CommandItem[][]>([])

  // Current level commands
  const currentCommands = computed(() => {
    if (commandStack.value.length > 0) {
      return commandStack.value[commandStack.value.length - 1]
    }
    return props.commands
  })

  // Filtered commands based on query
  const filteredCommands = computed(() => {
    if (!query.value.trim()) {
      return currentCommands.value
    }

    const q = query.value.toLowerCase()
    return currentCommands.value.filter(cmd => {
      const labelMatch = cmd.label.toLowerCase().includes(q)
      const descMatch = cmd.description?.toLowerCase().includes(q) ?? false
      return labelMatch || descMatch
    })
  })

  // Group commands by category
  const groupedCommands = computed(() => {
    const groups: Record<string, CommandItem[]> = {}

    filteredCommands.value.forEach(cmd => {
      const category = cmd.category || 'default'
      if (!groups[category]) {
        groups[category] = []
      }
      groups[category].push(cmd)
    })

    return groups
  })

  const flatCommands = computed(() => filteredCommands.value)

  const open = () => {
    visible.value = true
    query.value = ''
    selectedIndex.value = 0
    commandStack.value = []
    nextTick(() => {
      inputRef.value?.focus()
    })
  }

  const close = () => {
    visible.value = false
    query.value = ''
    selectedIndex.value = 0
    commandStack.value = []
    emit('close')
  }

  const goBack = () => {
    if (commandStack.value.length > 0) {
      commandStack.value.pop()
      query.value = ''
      selectedIndex.value = 0
    } else {
      close()
    }
  }

  const selectCommand = async (command: CommandItem) => {
    if (command.children && command.children.length > 0) {
      commandStack.value.push(command.children)
      query.value = ''
      selectedIndex.value = 0
      nextTick(() => {
        inputRef.value?.focus()
      })
    } else if (command.action) {
      close()
      await command.action()
    }
    emit('select', command)
  }

  const handleKeydown = (e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        selectedIndex.value = Math.min(selectedIndex.value + 1, flatCommands.value.length - 1)
        scrollToSelected()
        break
      case 'ArrowUp':
        e.preventDefault()
        selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
        scrollToSelected()
        break
      case 'Enter':
        e.preventDefault()
        if (flatCommands.value[selectedIndex.value]) {
          selectCommand(flatCommands.value[selectedIndex.value])
        }
        break
      case 'Escape':
        e.preventDefault()
        if (commandStack.value.length > 0) {
          goBack()
        } else {
          close()
        }
        break
      case 'Backspace':
        if (query.value === '' && commandStack.value.length > 0) {
          e.preventDefault()
          goBack()
        }
        break
    }
  }

  const scrollToSelected = () => {
    nextTick(() => {
      const selectedEl = listRef.value?.querySelector('.command-item--selected')
      selectedEl?.scrollIntoView({ block: 'nearest' })
    })
  }

  const handleGlobalKeydown = (e: KeyboardEvent) => {
    // ⌘K or Ctrl+K to open
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault()
      if (visible.value) {
        close()
      } else {
        open()
      }
    }
  }

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      close()
    }
  }

  watch(query, () => {
    selectedIndex.value = 0
  })

  onMounted(() => {
    document.addEventListener('keydown', handleGlobalKeydown)
  })

  onUnmounted(() => {
    document.removeEventListener('keydown', handleGlobalKeydown)
  })

  defineExpose({
    open,
    close,
  })
</script>

<template>
  <Teleport to="body">
    <Transition name="command-palette">
      <div v-if="visible" class="command-palette-overlay" @click="handleOverlayClick">
        <div class="command-palette">
          <div class="command-input-wrapper">
            <div v-if="commandStack.length > 0" class="command-breadcrumb" @click="goBack">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="15 18 9 12 15 6"></polyline>
              </svg>
            </div>
            <input
              ref="inputRef"
              v-model="query"
              type="text"
              class="command-input"
              :placeholder="placeholder || t('command_palette.placeholder', 'Type a command...')"
              @keydown="handleKeydown"
            />
            <div class="command-shortcut">
              <kbd>esc</kbd>
            </div>
          </div>

          <div ref="listRef" class="command-list">
            <template v-if="flatCommands.length === 0">
              <div class="command-empty">
                {{ t('command_palette.no_results', 'No results found') }}
              </div>
            </template>

            <template v-else>
              <template v-for="(commands, category) in groupedCommands" :key="category">
                <div v-if="category !== 'default'" class="command-category">
                  {{ category }}
                </div>
                <div
                  v-for="command in commands"
                  :key="command.id"
                  class="command-item"
                  :class="{ 'command-item--selected': flatCommands.indexOf(command) === selectedIndex }"
                  @click="selectCommand(command)"
                  @mouseenter="selectedIndex = flatCommands.indexOf(command)"
                >
                  <div class="command-item-content">
                    <span class="command-item-label">{{ command.label }}</span>
                    <span v-if="command.description" class="command-item-description">
                      {{ command.description }}
                    </span>
                  </div>
                  <div class="command-item-meta">
                    <kbd v-if="command.shortcut" class="command-item-shortcut">
                      {{ command.shortcut }}
                    </kbd>
                    <svg
                      v-if="command.children"
                      class="command-item-arrow"
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <polyline points="9 18 15 12 9 6"></polyline>
                    </svg>
                  </div>
                </div>
              </template>
            </template>
          </div>

          <div class="command-footer">
            <div class="command-footer-hint">
              <kbd>↑↓</kbd>
              {{ t('command_palette.navigate', 'navigate') }}
            </div>
            <div class="command-footer-hint">
              <kbd>↵</kbd>
              {{ t('command_palette.select', 'select') }}
            </div>
            <div class="command-footer-hint">
              <kbd>esc</kbd>
              {{ t('command_palette.close', 'close') }}
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
  .command-palette-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
    z-index: 9999;
  }

  .command-palette {
    width: 100%;
    max-width: 560px;
    background: var(--bg-200);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    box-shadow: 0 16px 70px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .command-input-wrapper {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-200);
    gap: 8px;
  }

  .command-breadcrumb {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    color: var(--text-400);
    cursor: pointer;
    border-radius: var(--border-radius-sm);
    transition: all 0.15s ease;
  }

  .command-breadcrumb:hover {
    color: var(--text-200);
    background: var(--bg-400);
  }

  .command-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-size: 15px;
    font-family: var(--font-family);
    color: var(--text-100);
  }

  .command-input::placeholder {
    color: var(--text-500);
  }

  .command-shortcut kbd {
    padding: 2px 6px;
    font-size: 11px;
    font-family: var(--font-family);
    color: var(--text-400);
    background: var(--bg-400);
    border-radius: var(--border-radius-xs);
  }

  .command-list {
    max-height: 320px;
    overflow-y: auto;
    padding: 8px;
  }

  .command-list::-webkit-scrollbar {
    width: 6px;
  }

  .command-list::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .command-category {
    padding: 8px 8px 4px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-500);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .command-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: background 0.1s ease;
  }

  .command-item:hover,
  .command-item--selected {
    background: var(--bg-400);
  }

  .command-item--selected {
    background: var(--color-primary-alpha);
  }

  .command-item-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .command-item-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-100);
  }

  .command-item-description {
    font-size: 12px;
    color: var(--text-400);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .command-item-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .command-item-shortcut {
    padding: 2px 6px;
    font-size: 11px;
    font-family: var(--font-family-mono);
    color: var(--text-400);
    background: var(--bg-500);
    border-radius: var(--border-radius-xs);
  }

  .command-item-arrow {
    color: var(--text-500);
  }

  .command-empty {
    padding: 24px;
    text-align: center;
    color: var(--text-400);
    font-size: 13px;
  }

  .command-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    border-top: 1px solid var(--border-200);
    background: var(--bg-300);
  }

  .command-footer-hint {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-500);
  }

  .command-footer-hint kbd {
    padding: 1px 4px;
    font-size: 10px;
    font-family: var(--font-family);
    color: var(--text-400);
    background: var(--bg-400);
    border-radius: 2px;
  }

  /* Transitions */
  .command-palette-enter-active,
  .command-palette-leave-active {
    transition: opacity 0.15s ease;
  }

  .command-palette-enter-active .command-palette,
  .command-palette-leave-active .command-palette {
    transition:
      transform 0.15s ease,
      opacity 0.15s ease;
  }

  .command-palette-enter-from,
  .command-palette-leave-to {
    opacity: 0;
  }

  .command-palette-enter-from .command-palette,
  .command-palette-leave-to .command-palette {
    transform: scale(0.96) translateY(-10px);
    opacity: 0;
  }
</style>
