import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useSessionStore } from '@/stores/session'
import { persistLeftSidebarState, restoreLeftSidebarState } from '@/persistence/session'

export type LeftSidebarPanel = 'workspace' | 'git' | 'config' | null

export const useLayoutStore = defineStore('layout', () => {
  const sessionStore = useSessionStore()

  const leftSidebarVisible = ref(false)
  const leftSidebarWidth = ref(280)
  const activeLeftPanel = ref<LeftSidebarPanel>('workspace')

  const isLeftSidebarOpen = computed(() => leftSidebarVisible.value && activeLeftPanel.value !== null)

  const isRestoring = ref(false)
  const isInitialized = ref(false)

  const setLeftSidebarWidth = (width: number) => {
    leftSidebarWidth.value = Math.max(200, Math.min(600, width))
  }

  const restoreFromSessionState = () => {
    const restored = restoreLeftSidebarState(sessionStore.uiState)
    if (!Object.keys(restored).length) return

    isRestoring.value = true
    try {
      if (typeof restored.leftSidebarVisible === 'boolean') leftSidebarVisible.value = restored.leftSidebarVisible
      if (typeof restored.leftSidebarWidth === 'number') setLeftSidebarWidth(restored.leftSidebarWidth)
      if (restored.leftSidebarActivePanel !== undefined) activeLeftPanel.value = restored.leftSidebarActivePanel
    } finally {
      isRestoring.value = false
    }
  }

  const saveToSessionState = () => {
    sessionStore.updateUiState(
      persistLeftSidebarState({
        leftSidebarVisible: leftSidebarVisible.value,
        leftSidebarWidth: leftSidebarWidth.value,
        leftSidebarActivePanel: activeLeftPanel.value,
      })
    )
  }

  const persist = () => {
    if (!isInitialized.value) return
    if (isRestoring.value) return
    saveToSessionState()
  }

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return
    await sessionStore.initialize()
    restoreFromSessionState()
    isInitialized.value = true
  }

  const toggleLeftSidebar = () => {
    leftSidebarVisible.value = !leftSidebarVisible.value
    persist()
  }

  const openLeftSidebar = () => {
    leftSidebarVisible.value = true
    persist()
  }

  const closeLeftSidebar = () => {
    leftSidebarVisible.value = false
    persist()
  }

  const setActivePanel = (panel: LeftSidebarPanel) => {
    if (panel === activeLeftPanel.value) {
      // 再次点击当前激活的 icon，收起次级面板
      activeLeftPanel.value = null
    } else {
      activeLeftPanel.value = panel
    }
    persist()
  }

  const setLeftSidebarWidthAndPersist = (width: number) => {
    setLeftSidebarWidth(width)
    persist()
  }

  // 内部拖拽路径
  const dragPath = ref<string | null>(null)

  const setDragPath = (path: string | null) => {
    dragPath.value = path
  }

  const consumeDragPath = () => {
    const path = dragPath.value
    dragPath.value = null
    return path
  }

  return {
    leftSidebarVisible,
    leftSidebarWidth,
    activeLeftPanel,
    isLeftSidebarOpen,
    isInitialized,
    initialize,
    toggleLeftSidebar,
    openLeftSidebar,
    closeLeftSidebar,
    setActivePanel,
    setLeftSidebarWidth: setLeftSidebarWidthAndPersist,
    dragPath,
    setDragPath,
    consumeDragPath,
  }
})
