<template>
  <div class="content-renderer" @dragover.prevent @drop="handleDrop">
    <EmptyState v-if="tabManagerStore.tabs.length === 0" />

    <Terminal
      v-for="tab in tabManagerStore.tabs.filter(t => t.type === TabType.TERMINAL)"
      v-show="tab.id === tabManagerStore.activeTabId"
      :key="tab.id"
      :terminal-id="Number(tab.id)"
      :is-active="tab.id === tabManagerStore.activeTabId"
      @input="handleInput"
      @resize="handleResize"
    />

    <SettingsView
      v-for="tab in tabManagerStore.tabs.filter(t => t.type === TabType.SETTINGS)"
      v-show="tab.id === tabManagerStore.activeTabId"
      :key="tab.id"
    />

    <DiffView
      v-for="tab in diffTabs"
      v-show="tab.id === tabManagerStore.activeTabId"
      :key="tab.id"
      :file-path="tab.data.filePath"
      :staged="tab.data.staged"
      :commit-hash="tab.data.commitHash"
    />
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { TabType, type DiffTabItem } from '@/types'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import Terminal from '@/components/terminal/Terminal.vue'
  import SettingsView from '@/views/Settings/SettingsView.vue'
  import EmptyState from '@/views/EmptyState/EmptyStateView.vue'
  import DiffView from '@/views/DiffView/DiffView.vue'

  const ORBITX_DRAG_PATH_MIME = 'application/x-orbitx-path'

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  const diffTabs = computed(() => {
    return tabManagerStore.tabs.filter((t): t is DiffTabItem => t.type === TabType.DIFF)
  })

  const handleInput = (data: string) => {
    if (typeof tabManagerStore.activeTabId === 'number') {
      terminalStore.writeToTerminal(tabManagerStore.activeTabId, data)
    }
  }

  const handleResize = (rows: number, cols: number) => {
    if (typeof tabManagerStore.activeTabId === 'number') {
      terminalStore.resizeTerminal(tabManagerStore.activeTabId, rows, cols)
    }
  }

  const handleDrop = (event: DragEvent) => {
    event.preventDefault()

    // 处理内部拖放（从 WorkspacePanel 拖入）
    const orbitXPath = event.dataTransfer?.getData(ORBITX_DRAG_PATH_MIME)
    if (orbitXPath) {
      insertPathToTerminal(orbitXPath)
      return
    }

    // 处理文本路径拖放
    const textPath = event.dataTransfer?.getData('text/plain')
    if (textPath && isAbsolutePath(textPath)) {
      insertPathToTerminal(textPath)
    }
  }

  const insertPathToTerminal = (path: string) => {
    if (typeof terminalStore.activeTerminalId !== 'number') return

    const cleanPath = path.trim().replace(/^["']|["']$/g, '')
    if (!cleanPath) return

    let processedPath = cleanPath
    if (cleanPath.includes(' ')) {
      processedPath = `"${cleanPath}"`
    }

    terminalStore.writeToTerminal(terminalStore.activeTerminalId, processedPath)
  }

  const isAbsolutePath = (value: string): boolean => {
    if (!value) return false
    if (value.startsWith('/')) return true
    if (/^[A-Za-z]:[\\/]/.test(value)) return true
    return false
  }
</script>

<style scoped>
  .content-renderer {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
