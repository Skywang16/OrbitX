<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import EditorArea from '@/components/editor/EditorArea.vue'
  import TitleBar from '@/components/ui/TitleBar.vue'
  import ActivityBar from '@/components/ui/ActivityBar.vue'
  import LeftSidebar from '@/components/ui/LeftSidebar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useEditorStore } from '@/stores/Editor'
  import { useLayoutStore } from '@/stores/layout'
  import { windowApi } from '@/api'
  import { onBeforeUnmount, onMounted } from 'vue'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import AIChatSidebar from '@/components/AIChatSidebar/index.vue'

  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()
  const aiChatStore = useAIChatStore()
  const layoutStore = useLayoutStore()

  let unlistenStartupFile: UnlistenFn | null = null
  let unlistenFileDropped: UnlistenFn | null = null

  const handleFilePath = async (filePath: string) => {
    const directory = await windowApi.handleFileOpen(filePath)
    await editorStore.createTerminalTab({ directory, activate: true })
  }

  onMounted(async () => {
    // 监听启动文件和应用图标拖放事件
    // 注意：窗口拖放事件由 Terminal.vue 中的 setupDragDropListener 处理，避免重复
    unlistenStartupFile = await windowApi.onStartupFile(handleFilePath)
    unlistenFileDropped = await windowApi.onFileDropped(handleFilePath)
  })

  onBeforeUnmount(() => {
    terminalStore.teardownGlobalListeners()

    if (unlistenStartupFile) {
      unlistenStartupFile()
    }
    if (unlistenFileDropped) {
      unlistenFileDropped()
    }

    // AI Chat 状态需要在卸载前同步到 SessionStore
    aiChatStore.saveToSessionState()
  })
</script>

<template>
  <div class="app-container">
    <TitleBar />

    <div class="main-content">
      <template v-if="layoutStore.leftSidebarVisible">
        <ActivityBar />
        <LeftSidebar v-if="layoutStore.activeLeftPanel" />
      </template>

      <EditorArea />

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
