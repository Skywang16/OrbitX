import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { TabType, type TabItem } from '@/types'
import { useTerminalStore } from './Terminal'

export const useTabManagerStore = defineStore('TabManager', () => {
  const tabs = ref<TabItem[]>([])
  const activeTabId = ref<string | null>(null)
  const terminalStore = useTerminalStore()

  const activeTab = computed(() => tabs.value.find(tab => tab.id === activeTabId.value))

  // 监听终端状态变化，自动同步标签
  watch(
    () => terminalStore.terminals,
    () => {
      syncTerminalTabs()
    },
    { deep: true }
  )

  const syncTerminalTabs = () => {
    tabs.value = tabs.value.filter(tab => tab.type !== TabType.TERMINAL)

    tabs.value.push(
      ...terminalStore.terminals.map(terminal => {
        const shellName = terminal.shell || 'shell'
        const cwd = terminal.cwd || '~'

        // 提取路径的最后一部分
        const pathParts = cwd.replace(/\/$/, '').split('/')
        const lastPath = pathParts[pathParts.length - 1] || '~'

        return {
          id: terminal.id,
          title: '', // 终端标签不再使用title字段
          type: TabType.TERMINAL,
          closable: true,
          icon: '🖥️',
          data: { backendId: terminal.backendId },
          shell: shellName,
          path: lastPath,
        }
      })
    )

    if (terminalStore.activeTerminalId) {
      activeTabId.value = terminalStore.activeTerminalId
    }
  }

  // --- 公共方法 ---

  const createSettingsTab = (section = 'theme'): string => {
    const existing = tabs.value.find(tab => tab.type === TabType.SETTINGS)
    if (existing) {
      setActiveTab(existing.id)
      return existing.id
    }

    const id = `settings-${Date.now()}`
    tabs.value.push({
      id,
      title: '设置',
      type: TabType.SETTINGS,
      closable: true,
      icon: '⚙️',
      data: { section },
    })
    setActiveTab(id)
    return id
  }

  const setActiveTab = (tabId: string) => {
    const tab = tabs.value.find(t => t.id === tabId)
    if (!tab) return

    activeTabId.value = tabId

    if (tab.type === TabType.TERMINAL) {
      terminalStore.setActiveTerminal(tabId)
    }
  }

  const closeTab = (tabId: string) => {
    const tabIndex = tabs.value.findIndex(tab => tab.id === tabId)
    if (tabIndex === -1) return

    const tab = tabs.value[tabIndex]

    if (tab.type === TabType.TERMINAL) {
      terminalStore.closeTerminal(tabId)
      return
    }

    tabs.value.splice(tabIndex, 1)

    if (activeTabId.value === tabId && tabs.value.length > 0) {
      const newIndex = Math.max(0, tabIndex - 1)
      setActiveTab(tabs.value[newIndex].id)
    } else if (tabs.value.length === 0) {
      activeTabId.value = null
    }
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    createSettingsTab,
    setActiveTab,
    closeTab,
    syncTerminalTabs,
    initialize: syncTerminalTabs,
  }
})
