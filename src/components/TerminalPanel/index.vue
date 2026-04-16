<script setup lang="ts">
  import Terminal from '@/components/terminal/Terminal.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { computed, onMounted, watch } from 'vue'

  const terminalStore = useTerminalStore()
  const workspaceStore = useWorkspaceStore()
  let hasSeededInitialTerminal = false

  const activeTerminal = computed(() => terminalStore.activeTerminal)
  const terminalTabs = computed(() =>
    [...terminalStore.terminals].sort((a, b) => {
      if (a.kind !== b.kind) {
        return a.kind === 'workspace' ? -1 : 1
      }
      return a.id - b.id
    })
  )

  const ensureTerminalSelection = async (createWhenEmpty: boolean = false) => {
    if (terminalTabs.value.length > 0) {
      if (terminalStore.activeTerminalId === null) {
        await terminalStore.setActiveTerminal(terminalTabs.value[0].id)
      }
      return
    }

    if (!createWhenEmpty) {
      return
    }

    const id = await terminalStore.createTerminalPane(workspaceStore.currentWorkspacePath || undefined)
    await terminalStore.setActiveTerminal(id)
    hasSeededInitialTerminal = true
  }

  onMounted(async () => {
    await terminalStore.initializeTerminalStore()
    await ensureTerminalSelection(!hasSeededInitialTerminal)
  })

  watch(
    () => [terminalTabs.value.map(terminal => terminal.id).join(','), terminalStore.activeTerminalId] as const,
    async () => {
      await ensureTerminalSelection()
    }
  )

  const createWorkspaceTerminal = async () => {
    const terminalId = await terminalStore.createTerminalPane(workspaceStore.currentWorkspacePath || undefined)
    await terminalStore.setActiveTerminal(terminalId)
  }

  const closeTerminal = async (terminalId: number) => {
    await terminalStore.closeTerminal(terminalId)
    await ensureTerminalSelection()
  }

  const getKindLabel = (kind: 'workspace' | 'agent') => (kind === 'agent' ? 'Agent' : 'Shell')

  const getStatusLabel = (status: string | null) => {
    if (!status) return null
    return status[0].toUpperCase() + status.slice(1)
  }
</script>

<template>
  <div class="terminal-panel">
    <div class="terminal-sidebar">
      <div class="terminal-sidebar__header">
        <span class="terminal-sidebar__title">Terminal</span>
        <button class="terminal-new" @click="createWorkspaceTerminal">+</button>
      </div>

      <div class="terminal-tabs">
        <div
          v-for="terminal in terminalTabs"
          :key="terminal.id"
          class="terminal-tab"
          :class="{ 'terminal-tab--active': terminal.id === terminalStore.activeTerminalId }"
          role="button"
          tabindex="0"
          @click="terminalStore.setActiveTerminal(terminal.id)"
          @keydown.enter.prevent="terminalStore.setActiveTerminal(terminal.id)"
          @keydown.space.prevent="terminalStore.setActiveTerminal(terminal.id)"
        >
          <div class="terminal-tab__body">
            <div class="terminal-tab__topline">
              <span class="terminal-tab__title">{{ terminal.displayTitle }}</span>
              <span class="terminal-badge">{{ getKindLabel(terminal.kind) }}</span>
            </div>
            <div class="terminal-tab__subline">
              <span v-if="terminal.sourceLabel && terminal.kind === 'agent'" class="terminal-source">
                {{ terminal.sourceLabel }}
              </span>
              <span v-else class="terminal-source">{{ terminal.cwd }}</span>
              <span v-if="getStatusLabel(terminal.agentStatus)" class="terminal-status">
                {{ getStatusLabel(terminal.agentStatus) }}
              </span>
            </div>
          </div>
          <button class="terminal-tab__close" @click.stop="closeTerminal(terminal.id)">×</button>
        </div>
      </div>
    </div>

    <div class="terminal-content">
      <Terminal
        v-if="activeTerminal"
        :terminal-id="activeTerminal.id"
        :is-active="activeTerminal.id === terminalStore.activeTerminalId"
      />
      <div v-else class="loading-state">
        <div class="empty-state">
          <span>No terminals running.</span>
          <button class="terminal-new terminal-new--empty" @click="createWorkspaceTerminal">Create Terminal</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .terminal-panel {
    display: flex;
    height: 100%;
    background: var(--bg-200);
    position: relative;
    min-height: 0;
  }

  .terminal-sidebar {
    display: flex;
    flex-direction: column;
    width: 248px;
    min-width: 208px;
    border-right: 1px solid var(--border-200);
    background: color-mix(in srgb, var(--bg-100) 88%, transparent);
  }

  .terminal-sidebar__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-200);
  }

  .terminal-sidebar__title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-300);
  }

  .terminal-tabs {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    min-height: 0;
    padding: 8px;
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .terminal-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--border-200);
    border-radius: 8px;
    background: transparent;
    color: var(--text-300);
    cursor: pointer;
    text-align: left;
    min-width: 0;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      color 0.15s ease;
  }

  .terminal-tab:hover {
    background: color-mix(in srgb, var(--bg-300) 65%, transparent);
  }

  .terminal-tab--active {
    background: var(--color-primary-alpha, color-mix(in srgb, var(--color-primary) 10%, transparent));
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-200));
    color: var(--color-primary);
  }

  .terminal-tab__body {
    flex: 1;
    min-width: 0;
  }

  .terminal-tab__topline,
  .terminal-tab__subline {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .terminal-tab__subline {
    margin-top: 3px;
  }

  .terminal-tab__title {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .terminal-badge,
  .terminal-status {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 999px;
    font-size: 10px;
    background: color-mix(in srgb, var(--bg-400) 82%, transparent);
    color: var(--text-300);
    flex-shrink: 0;
  }

  .terminal-source {
    font-size: 11px;
    color: var(--text-400);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
  }

  .terminal-tab__close,
  .terminal-new {
    border: none;
    border-radius: 8px;
    background: var(--bg-400);
    color: var(--text-200);
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .terminal-tab__close {
    width: 24px;
    height: 24px;
    flex-shrink: 0;
  }

  .terminal-new {
    width: 28px;
    height: 28px;
    padding: 0;
    font-size: 18px;
    font-weight: 600;
    line-height: 1;
  }

  .terminal-tab__close:hover,
  .terminal-new:hover {
    background: var(--color-hover);
    color: var(--color-primary);
  }

  .terminal-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-400);
    font-size: 13px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .terminal-new--empty {
    width: auto;
    height: 32px;
    padding: 0 14px;
    font-size: 12px;
  }

  @media (max-width: 900px) {
    .terminal-panel {
      flex-direction: column;
    }

    .terminal-sidebar {
      width: 100%;
      min-width: 0;
      border-right: none;
      border-bottom: 1px solid var(--border-200);
    }

    .terminal-tabs {
      max-height: 132px;
    }
  }
</style>
