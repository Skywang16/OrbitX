<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import { TabType } from '@/types'

  interface Props {
    selectedText?: string
    selectionInfo?: string
    visible?: boolean
  }

  const props = withDefaults(defineProps<Props>(), {
    selectedText: '',
    selectionInfo: '',
    visible: false,
  })

  const emit = defineEmits<{
    clear: []
    insert: []
  }>()

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()
  const { t } = useI18n()

  // 获取当前活跃tab的路径信息
  const currentTabPath = computed(() => {
    const activeTab = tabManagerStore.activeTab
    if (!activeTab || activeTab.type !== TabType.TERMINAL) {
      return 'terminal'
    }

    // 优先使用tab中的path信息
    if (activeTab.path && activeTab.path !== '~') {
      return activeTab.path
    }

    // 如果tab没有path信息，从terminal store获取
    const terminal = terminalStore.terminals.find(t => t.id === activeTab.id)
    if (terminal?.cwd) {
      // 使用简化的路径显示逻辑
      const parts = terminal.cwd
        .replace(/\/$/, '')
        .split(/[/\\]/)
        .filter(p => p.length > 0)
      if (parts.length === 0) return '~'

      const lastPart = parts[parts.length - 1]
      return lastPart.length > 15 ? lastPart.substring(0, 12) + '...' : lastPart
    }

    return 'terminal'
  })

  // 简化显示文本逻辑 - 优先显示路径信息
  const displayText = computed(() => {
    if (props.selectionInfo) {
      // 如果有选择信息，显示 "路径 行号:行号" 格式
      const parts = props.selectionInfo.split(' ')
      if (parts.length > 1) {
        return `${currentTabPath.value} ${parts.slice(1).join(' ')}`
      }
    }
    return `${currentTabPath.value} ${t('session.selected_content')}`
  })
</script>

<template>
  <div v-if="visible && selectedText" class="terminal-selection-tag">
    <div class="tag-content">
      <div class="tag-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
          <line x1="8" y1="21" x2="16" y2="21" />
          <line x1="12" y1="17" x2="12" y2="21" />
        </svg>
      </div>
      <span class="tag-text" @click="emit('insert')" :title="`${t('session.click_to_insert')}: ${displayText}`">
        {{ displayText }}
      </span>
      <button class="tag-close" @click="emit('clear')" :title="t('session.clear_selection')">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .terminal-selection-tag {
    margin-bottom: 8px;
  }

  .tag-content {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background-color: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text-300);
    max-width: 100%;
  }

  .tag-icon,
  .tag-close {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tag-icon {
    color: var(--color-primary);
  }

  .tag-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    cursor: pointer;
    transition: color 0.1s ease;
  }

  .tag-text:hover {
    color: var(--color-primary);
  }

  .tag-close {
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-400);
    cursor: pointer;
    padding: 2px;
    border-radius: 50%;
    transition: all 0.1s ease;
  }

  .tag-close:hover {
    background-color: var(--bg-600);
    color: var(--text-200);
  }

  .tag-close:active {
    transform: scale(0.95);
  }
</style>
