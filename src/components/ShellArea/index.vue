<script setup lang="ts">
  import Terminal from '@/components/terminal/Terminal.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  interface Props {
    threadId: number
    workspacePath: string
  }

  const props = defineProps<Props>()
  const terminalStore = useTerminalStore()

  // Shell thread's terminal ID (one per shell thread)
  const shellTerminalId = ref<number | null>(null)

  // Get terminal display title
  const shellTerminal = computed(() => {
    if (shellTerminalId.value === null) return null
    return terminalStore.terminals.find(t => t.id === shellTerminalId.value)
  })

  const shellTitle = computed(() => {
    return shellTerminal.value?.displayTitle || props.workspacePath.split('/').pop() || props.workspacePath
  })

  // Ensure terminal exists for this shell thread
  const ensureShellTerminal = async () => {
    const existingTerminal = terminalStore.terminals.find(t => t.threadId === props.threadId && t.kind === 'workspace')

    if (existingTerminal) {
      shellTerminalId.value = existingTerminal.id
      await terminalStore.setActiveTerminal(existingTerminal.id)
      return
    }

    const paneId = await terminalStore.createTerminalPane(props.workspacePath, {
      threadId: props.threadId,
    })
    shellTerminalId.value = paneId
    await terminalStore.setActiveTerminal(paneId)
  }

  // Window drag and maximize
  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  const handleDoubleClick = async () => {
    const win = getCurrentWindow()
    const isMaximized = await win.isMaximized()
    if (isMaximized) {
      await win.unmaximize()
    } else {
      await win.maximize()
    }
  }

  onMounted(async () => {
    await ensureShellTerminal()
  })

  // Watch for thread changes - re-create terminal if thread switches
  watch(
    () => props.threadId,
    async (newThreadId, oldThreadId) => {
      if (newThreadId !== oldThreadId) {
        await ensureShellTerminal()
      }
    }
  )

  onUnmounted(() => {
    // Keep terminal running when switching away - user may have running commands
    // Terminal will be cleaned up when thread is deleted
  })
</script>

<template>
  <div class="shell-area">
    <!-- Header with drag region -->
    <div class="shell-header" @mousedown="startDrag" @dblclick="handleDoubleClick">
      <div class="header-left" @mousedown.stop @dblclick.stop>
        <span class="shell-title">{{ shellTitle }}</span>
      </div>
      <div class="header-center" data-tauri-drag-region />
      <div class="header-right" @mousedown.stop @dblclick.stop>
        <!-- Optional: Add action buttons here -->
      </div>
    </div>

    <!-- Terminal content -->
    <div class="shell-terminal">
      <Terminal v-if="shellTerminalId !== null" :terminal-id="shellTerminalId" :is-active="true" />
      <div v-else class="loading-state">
        <span>Initializing shell...</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .shell-area {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-100);
    border-radius: var(--border-radius-2xl);
    overflow: hidden;
  }

  .shell-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 20px 0 16px;
    flex-shrink: 0;
    position: relative;
    z-index: 10;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .shell-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .header-center {
    flex: 1;
    -webkit-app-region: drag;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .shell-terminal {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 4px 8px 8px;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-400);
    font-size: 13px;
  }
</style>
