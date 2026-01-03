<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import ContentRenderer from '@/components/ui/ContentRenderer.vue'
  import TitleBar from '@/components/ui/TitleBar.vue'
  import ActivityBar from '@/components/ui/ActivityBar.vue'
  import LeftSidebar from '@/components/ui/LeftSidebar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useLayoutStore } from '@/stores/layout'
  import { windowApi } from '@/api'
  import { onBeforeUnmount, onMounted } from 'vue'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import AIChatSidebar from '@/components/AIChatSidebar/index.vue'

  const terminalStore = useTerminalStore()
  const aiChatStore = useAIChatStore()
  const tabManagerStore = useTabManagerStore()
  const layoutStore = useLayoutStore()

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
    // 监听启动文件和应用图标拖放事件
    const handleAppIconFileDrop = (filePath: string) => {
      handleFilePath(filePath, 'app-icon')
    }

    unlistenStartupFile = await windowApi.onStartupFile(handleAppIconFileDrop)
    unlistenFileDropped = await windowApi.onFileDropped(handleAppIconFileDrop)

    // 监听窗口拖放事件
    unlistenFileDrop = await windowApi.onWindowDragDrop(filePath => {
      handleFilePath(filePath, 'window')
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

    // AI Chat 状态需要在卸载前同步到 SessionStore
    aiChatStore.saveToSessionState()
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
      <template v-if="layoutStore.leftSidebarVisible">
        <ActivityBar />
        <LeftSidebar v-if="layoutStore.activeLeftPanel" />
      </template>

      <ContentRenderer />

      <div
        v-show="aiChatStore.isVisible"
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
