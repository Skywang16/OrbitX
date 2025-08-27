<template>
  <div class="content-renderer">
    <!-- 当没有任何tab时显示空状态页面 -->
    <EmptyState v-if="tabManagerStore.tabs.length === 0" />

    <!-- 终端标签页 -->
    <Terminal
      v-for="tab in tabManagerStore.tabs.filter(t => t.type === TabType.TERMINAL)"
      v-show="tab.id === tabManagerStore.activeTabId"
      :key="tab.id"
      :terminal-id="tab.id"
      :backend-id="terminalStore.terminals.find(t => t.id === tab.id)?.backendId || null"
      :is-active="tab.id === tabManagerStore.activeTabId"
      @input="handleInput"
      @resize="handleResize"
    />

    <!-- 设置页面 -->
    <SettingsView
      v-for="tab in tabManagerStore.tabs.filter(t => t.type === TabType.SETTINGS)"
      v-show="tab.id === tabManagerStore.activeTabId"
      :key="tab.id"
    />
  </div>
</template>

<script setup lang="ts">
  import { TabType } from '@/types'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import Terminal from '@/components/terminal/Terminal.vue'
  import SettingsView from '@/views/Settings/SettingsView.vue'
  import EmptyState from '@/components/ui/EmptyState.vue'

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  const handleInput = (data: string) => {
    if (tabManagerStore.activeTabId) terminalStore.writeToTerminal(tabManagerStore.activeTabId, data)
  }

  const handleResize = (rows: number, cols: number) => {
    if (tabManagerStore.activeTabId) terminalStore.resizeTerminal(tabManagerStore.activeTabId, rows, cols)
  }
</script>

<style scoped>
  .content-renderer {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-200);
  }
</style>
