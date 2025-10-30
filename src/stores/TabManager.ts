import { defineStore } from 'pinia'
import { computed, watch } from 'vue'
import { TabType, type TabItem } from '@/types'
import { useTerminalStore } from './Terminal'
import { useSessionStore } from './session'
import { dockApi } from '@/api'
import { getHandler, defaultCloseTab } from './TabHandlers'

export const useTabManagerStore = defineStore('TabManager', () => {
  const terminalStore = useTerminalStore()
  const sessionStore = useSessionStore()

  const tabs = computed<TabItem[]>(() => {
    return sessionStore.tabs.map(tab => getHandler(tab.type).buildTabItem(tab))
  })

  const activeTabId = computed(() => sessionStore.activeTabId)
  const activeTab = computed(() => tabs.value.find(tab => tab.id === activeTabId.value))

  // 监听终端列表和活动标签页变化时更新Dock菜单
  watch(
    [() => terminalStore.terminals, () => activeTabId.value],
    () => {
      updateDockMenu()
    },
    { deep: true }
  )

  const createSettingsTab = (): number => {
    const existing = sessionStore.tabs.find(tab => tab.type === 'settings')
    if (existing) {
      setActiveTab(existing.id)
      return existing.id
    }

    const id = -1
    const newTab = {
      type: 'settings' as const,
      id,
      isActive: false,
      data: {
        lastSection: 'general',
      },
    }

    sessionStore.updateTabs([...sessionStore.tabs, newTab])
    setActiveTab(id)

    return id
  }

  const updateSettingsTabSection = (tabId: number, section: string) => {
    const updatedTabs = sessionStore.tabs.map(tab => {
      if (tab.type === 'settings' && tab.id === tabId) {
        return {
          ...tab,
          data: {
            ...tab.data,
            lastSection: section,
          },
        }
      }
      return tab
    })
    sessionStore.updateTabs(updatedTabs)
  }

  const getSettingsTabSection = (tabId: number): string | undefined => {
    const tab = sessionStore.tabs.find(t => t.type === 'settings' && t.id === tabId)
    if (tab && tab.type === 'settings') {
      return tab.data.lastSection
    }
    return undefined
  }

  const setActiveTab = async (tabId: number) => {
    const tab = sessionStore.tabs.find(t => t.id === tabId)
    if (!tab) return
    await getHandler(tab.type).activate(tabId)
  }

  const closeTab = async (tabId: number) => {
    const tab = sessionStore.tabs.find(t => t.id === tabId)
    if (!tab) return

    const handler = getHandler(tab.type)
    if (handler.close) {
      await handler.close(tabId)
    } else {
      await defaultCloseTab(tabId)
    }
  }

  const closeLeftTabs = async (currentTabId: number) => {
    const currentIndex = tabs.value.findIndex(tab => tab.id === currentTabId)
    if (currentIndex <= 0) return

    const idsToClose = tabs.value
      .slice(0, currentIndex)
      .filter(t => t.closable)
      .map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  const closeRightTabs = async (currentTabId: number) => {
    const currentIndex = tabs.value.findIndex(tab => tab.id === currentTabId)
    if (currentIndex === -1 || currentIndex >= tabs.value.length - 1) return

    const idsToClose = tabs.value
      .slice(currentIndex + 1)
      .filter(t => t.closable)
      .map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  const closeOtherTabs = async (currentTabId: number) => {
    const idsToClose = tabs.value.filter(tab => tab.id !== currentTabId && tab.closable).map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  const closeAllTabs = async () => {
    const idsToClose = tabs.value.filter(tab => tab.closable).map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  const updateDockMenu = () => {
    const tabEntries = tabs.value
      .filter(tab => tab.type === TabType.TERMINAL)
      .map(tab => ({
        id: String(tab.id),
        title: tab.path || tab.title || 'Terminal',
      }))

    const activeId = activeTabId.value !== null ? String(activeTabId.value) : null

    dockApi.updateTabs(tabEntries, activeId)
  }

  const initialize = async () => {
    if (!sessionStore.initialized) {
      await sessionStore.initialize()
    }
  }

  return {
    // 计算属性
    tabs,
    activeTabId,
    activeTab,

    // Settings tab 操作
    createSettingsTab,
    updateSettingsTabSection,
    getSettingsTabSection,

    // Tab 操作
    setActiveTab,
    closeTab,
    closeLeftTabs,
    closeRightTabs,
    closeOtherTabs,
    closeAllTabs,

    // 工具方法
    initialize,
  }
})
