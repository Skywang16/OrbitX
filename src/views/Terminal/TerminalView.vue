<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import ContentRenderer from '@/components/ui/ContentRenderer.vue'
  import TitleBar from '@/components/ui/TitleBar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { windowApi } from '@/api'
  import { listen, UnlistenFn } from '@tauri-apps/api/event'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { onBeforeUnmount, onMounted, watch } from 'vue'
  import AIChatSidebar from '@/components/AIChatSidebar/index.vue'

  const terminalStore = useTerminalStore()
  const aiChatStore = useAIChatStore()
  const tabManagerStore = useTabManagerStore()

  // 存储事件监听器的取消函数
  let unlistenStartupFile: UnlistenFn | null = null
  let unlistenFileDropped: UnlistenFn | null = null
  let unlistenFileDrop: UnlistenFn | null = null

  /**
   * 处理文件路径，根据来源决定行为
   */
  const handleFilePath = async (filePath: string, source: 'app-icon' | 'window' = 'app-icon') => {
    try {
      if (source === 'app-icon') {
        // 拖动到应用图标：新建终端tab并定位到文件所在目录
        const directory = await windowApi.handleFileOpen(filePath)
        await terminalStore.createTerminal(directory)
      } else {
        // 拖动到窗口内：将文件路径插入到当前终端输入行
        insertFilePathToCurrentTerminal(filePath)
      }
    } catch (error) {
      console.warn('处理文件路径失败:', error)
    }
  }

  /**
   * 将文件路径插入到当前活跃终端
   */
  const insertFilePathToCurrentTerminal = (filePath: string) => {
    if (!terminalStore.activeTerminalId) return

    // 处理路径中的空格，添加引号
    let processedPath = filePath
    if (filePath.includes(' ')) {
      processedPath = `"${filePath}"`
    }

    // 直接发送到当前终端
    terminalStore.writeToTerminal(terminalStore.activeTerminalId, processedPath)
  }

  watch(
    () => terminalStore.terminals,
    () => {
      tabManagerStore.syncTerminalTabs()
    },
    { deep: true }
  )

  // 当主应用组件挂载时，初始化应用状态
  onMounted(async () => {
    // 统一的文件处理函数
    const handleAppIconFileDrop = (event: { payload: string }) => {
      handleFilePath(event.payload, 'app-icon')
    }

    // 监听应用启动时的文件参数和文件拖拽事件（合并处理）
    unlistenStartupFile = await listen<string>('startup-file', handleAppIconFileDrop)
    unlistenFileDropped = await listen<string>('file-dropped', handleAppIconFileDrop)

    // 监听 Tauri 原生文件拖拽事件（窗口内拖拽）
    const webview = getCurrentWebview()
    unlistenFileDrop = await webview.onDragDropEvent(event => {
      // 只处理文件拖拽放置事件
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

  // 应用关闭/卸载时清理监听器并保存状态
  onBeforeUnmount(() => {
    // 先立即清理监听器，确保不阻塞关闭
    terminalStore.teardownGlobalListeners()

    // 清理文件拖拽事件监听器
    if (unlistenStartupFile) {
      unlistenStartupFile()
    }
    if (unlistenFileDropped) {
      unlistenFileDropped()
    }
    if (unlistenFileDrop) {
      unlistenFileDrop()
    }

    // 异步保存状态，不阻塞关闭流程
    Promise.resolve().then(async () => {
      try {
        aiChatStore.saveToSessionState()

        await terminalStore.saveSessionState()
      } catch (error) {
        console.error(' [TerminalView] 状态保存失败:', error)
        // 保存失败不影响应用关闭
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
      <!-- 使用新的内容渲染器 -->
      <ContentRenderer />

      <!-- AI聊天侧边栏 -->
      <div v-if="aiChatStore.isVisible" class="sidebar-wrapper" :style="{ width: `${aiChatStore.sidebarWidth}px` }">
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
    flex-shrink: 0;
    min-height: 0;
    overflow: hidden;
  }
</style>
