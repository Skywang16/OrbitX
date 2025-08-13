<template>
  <div class="content-renderer">
    <Terminal
      v-for="tab in terminalTabs"
      v-show="tab.id === activeTabId"
      :key="tab.id"
      :terminal-id="tab.id"
      :backend-id="getTerminal(tab.id)?.backendId || null"
      :is-active="tab.id === activeTabId"
      @input="handleInput"
      @resize="handleResize"
    />

    <SettingsView
      v-for="tab in settingsTabs"
      v-show="tab.id === activeTabId"
      :key="tab.id"
      :section="tab.data?.section || 'theme'"
    />
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { TabType } from '@/types'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import Terminal from '@/components/terminal/Terminal.vue'
  import SettingsView from '@/views/Settings/SettingsView.vue'

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  const activeTabId = computed(() => tabManagerStore.activeTabId)
  const terminalTabs = computed(() => tabManagerStore.tabs.filter(tab => tab.type === TabType.TERMINAL))
  const settingsTabs = computed(() => tabManagerStore.tabs.filter(tab => tab.type === TabType.SETTINGS))

  const getTerminal = (id: string) => terminalStore.terminals.find(t => t.id === id)

  const handleInput = (data: string) => {
    if (activeTabId.value) terminalStore.writeToTerminal(activeTabId.value, data)
  }

  const handleResize = (rows: number, cols: number) => {
    if (activeTabId.value) terminalStore.resizeTerminal(activeTabId.value, rows, cols)
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
