<script setup lang="ts">
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import AIChatSidebar from '@/components/AIChatSidebar/index.vue'
  import ContentRenderer from '@/components/ui/ContentRenderer.vue'
  import TitleBar from '@/components/ui/TitleBar.vue'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { invoke } from '@tauri-apps/api/core'
  import { listen, UnlistenFn } from '@tauri-apps/api/event'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { onBeforeUnmount, onMounted, watch } from 'vue'

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
        const directory = await invoke<string>('handle_file_open', { path: filePath })
        await terminalStore.createTerminal(directory)
      } else {
        // 拖动到窗口内：将文件路径插入到当前终端输入行
        insertFilePathToCurrentTerminal(filePath)
      }
    } catch (error) {
      console.error('处理文件路径失败:', error)
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

  // 监听终端状态变化，同步到标签管理器
  watch(
    () => terminalStore.terminals,
    () => {
      tabManagerStore.syncTerminalTabs()
    },
    { deep: true }
  )

  watch(
    () => terminalStore.activeTerminalId,
    newActiveId => {
      if (newActiveId && tabManagerStore.activeTabId !== newActiveId) {
        tabManagerStore.setActiveTab(newActiveId)
      }
    }
  )

  // 当主应用组件挂载时，设置全局监听器
  onMounted(async () => {
    await terminalStore.setupGlobalListeners()

    // 初始化shell管理器
    await terminalStore.initializeShellManager()

    // 初始化标签管理器
    tabManagerStore.initialize()

    // 如果没有终端，创建一个初始终端
    if (terminalStore.terminals.length === 0) {
      await terminalStore.createTerminal()
    }

    // 监听应用启动时的文件参数（拖动到应用图标）
    unlistenStartupFile = await listen<string>('startup-file', event => {
      handleFilePath(event.payload, 'app-icon')
    })

    // 监听文件拖拽事件（从single instance插件，拖动到应用图标）
    unlistenFileDropped = await listen<string>('file-dropped', event => {
      handleFilePath(event.payload, 'app-icon')
    })

    // 监听 Tauri 原生文件拖拽事件
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

  // 应用关闭/卸载时清理监听器
  onBeforeUnmount(() => {
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
    background-color: var(--color-background);
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
