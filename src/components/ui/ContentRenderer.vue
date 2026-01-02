<template>
  <div class="content-renderer">
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
</script>

<style scoped>
  .content-renderer {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
