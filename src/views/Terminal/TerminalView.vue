<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import ContentRenderer from '@/components/ui/ContentRenderer.vue'
  import TitleBar from '@/components/ui/TitleBar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { windowApi } from '@/api'
  import { listen, UnlistenFn } from '@tauri-apps/api/event'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { onBeforeUnmount, onMounted } from 'vue'
  import AIChatSidebar from '@/components/AIChatSidebar/index.vue'

  const terminalStore = useTerminalStore()
  const aiChatStore = useAIChatStore()
  const tabManagerStore = useTabManagerStore()

  let unlistenStartupFile: UnlistenFn | null = null
  let unlistenFileDropped: UnlistenFn | null = null
  let unlistenFileDrop: UnlistenFn | null = null

  const handleFilePath = async (filePath: string, source: 'app-icon' | 'window' = 'app-icon') => {
    if (source === 'app-icon') {
      const directory = await windowApi.handleFileOpen(filePath)
      await terminalStore.createTerminal(directory)
    } else {
      insertFilePathToCurrentTerminal(filePath)
    }
  }

  const insertFilePathToCurrentTerminal = (filePath: string) => {
    if (typeof terminalStore.activeTerminalId !== 'number') return

    let processedPath = filePath
    if (filePath.includes(' ')) {
      processedPath = `"${filePath}"`
    }

    terminalStore.writeToTerminal(terminalStore.activeTerminalId, processedPath)
  }

  onMounted(async () => {
    const handleAppIconFileDrop = (event: { payload: string }) => {
      handleFilePath(event.payload, 'app-icon')
    }

    unlistenStartupFile = await listen<string>('startup-file', handleAppIconFileDrop)
    unlistenFileDropped = await listen<string>('file-dropped', handleAppIconFileDrop)

    const webview = getCurrentWebview()
    unlistenFileDrop = await webview.onDragDropEvent(event => {
      if (
        event.event === 'tauri://drag-drop' &&
        event.payload &&
        'paths' in event.payload &&
        event.payload.paths &&
        event.payload.paths.length > 0
      ) {
        const filePath = event.payload.paths[0]
        handleFilePath(filePath, 'window')
      }
    })
  })

  onBeforeUnmount(() => {
    terminalStore.teardownGlobalListeners()

    if (unlistenStartupFile) {
      unlistenStartupFile()
    }
    if (unlistenFileDropped) {
      unlistenFileDropped()
    }
    if (unlistenFileDrop) {
      unlistenFileDrop()
    }

    Promise.resolve().then(async () => {
      try {
        aiChatStore.saveToSessionState()

        await terminalStore.saveSessionState()
      } catch (error) {
        console.warn('Failed to save session state:', error)
      }
    })
  })
</script>

<template>
  <div class="app-container">
    <TitleBar
      :tabs="tabManagerStore.tabs"
      :activeTabId="tabManagerStore.activeTabId"
      @switch="tabManagerStore.setActiveTab"
      @close="tabManagerStore.closeTab"
    />

    <div class="main-content">
      <ContentRenderer />

      <div
        v-if="aiChatStore.isVisible"
        class="sidebar-wrapper"
        :style="{ '--sidebar-width': `${aiChatStore.sidebarWidth}px` }"
      >
        <AIChatSidebar />
      </div>
    </div>
  </div>
</template>

<style scoped>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: var(--bg-200);
  }

  .main-content {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar-wrapper {
    flex: 0 1 auto;
    flex-basis: var(--sidebar-width);
    max-width: 70vw;
    min-width: 10vw;
    min-height: 0;
    overflow: hidden;
  }
</style>
