import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { TabType, type TabItem } from '@/types'
import { useTerminalStore } from './Terminal'

export const useTabManagerStore = defineStore('TabManager', () => {
  const tabs = ref<TabItem[]>([])
  const activeTabId = ref<string | null>(null)
  const terminalStore = useTerminalStore()

  const activeTab = computed(() => tabs.value.find(tab => tab.id === activeTabId.value))

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
        const displayPath = getDisplayPath(cwd)

        return {
          id: terminal.id,
          title: '',
          type: TabType.TERMINAL,
          closable: true,
          icon: '🖥️',
          data: { backendId: terminal.backendId },
          shell: shellName,
          path: displayPath,
        }
      })
    )

    if (terminalStore.activeTerminalId) {
      activeTabId.value = terminalStore.activeTerminalId
    }
  }

  /** 格式化工作目录为短显示名 */
  const getDisplayPath = (cwd: string): string => {
    if (!cwd || cwd === '~') return '~'

    try {
      const cleanPath = cwd.replace(/\/$/, '')
      const homePatterns = [/^\/Users\/[^/]+/, /^\/home\/[^/]+/, /^C:\\Users\\[^\\]+/i]

      for (const homePattern of homePatterns) {
        if (homePattern.test(cleanPath)) {
          const homeMatch = cleanPath.match(homePattern)?.[0]
          if (homeMatch && cleanPath === homeMatch) {
            return '~'
          }
          const relativePath = cleanPath.replace(homePattern, '~')
          const pathParts = relativePath.split(/[/\\]/).filter(p => p.length > 0)
          if (pathParts.length > 0) {
            const lastPart = pathParts[pathParts.length - 1]
            return lastPart.length > 20 ? lastPart.substring(0, 17) + '...' : lastPart
          }
          return '~'
        }
      }
      const systemDirs: Record<string, string> = {
        '/': 'root',
        '/usr': 'usr',
        '/etc': 'etc',
        '/var': 'var',
        '/tmp': 'tmp',
        '/opt': 'opt',
        '/Applications': 'Apps',
        '/System': 'System',
        '/Library': 'Library',
        'C:\\': 'C:',
        'D:\\': 'D:',
      }

      if (systemDirs[cleanPath]) {
        return systemDirs[cleanPath]
      }
      const pathParts = cleanPath.split(/[/\\]/).filter(p => p.length > 0)

      if (pathParts.length === 0) return '/'

      const lastPart = pathParts[pathParts.length - 1]
      if (pathParts.length === 1 && (cleanPath.startsWith('/') || cleanPath.match(/^[A-Z]:\\/i))) {
        return navigator.platform.toLowerCase().includes('win') ? lastPart : `/${lastPart}`
      }
      if (lastPart.length > 20) {
        return lastPart.substring(0, 17) + '...'
      }

      return lastPart
    } catch (error) {
      console.warn('路径处理错误:', error, '原始路径:', cwd)
      const parts = cwd.split(/[/\\]/).filter(p => p.length > 0)
      return parts.length > 0 ? parts[parts.length - 1] : '~'
    }
  }

  const createSettingsTab = (): string => {
    const existing = tabs.value.find(tab => tab.type === TabType.SETTINGS)
    if (existing) {
      setActiveTab(existing.id)
      return existing.id
    }

    const id = `settings-${Date.now()}`
    tabs.value.push({
      id,
      title: 'settings',
      type: TabType.SETTINGS,
      closable: true,
      data: {},
    })
    setActiveTab(id)
    return id
  }

  const createLLMTestTab = (): string => {
    const existing = tabs.value.find(tab => tab.type === TabType.LLM_TEST)
    if (existing) {
      setActiveTab(existing.id)
      return existing.id
    }

    const id = `llm-test-${Date.now()}`
    tabs.value.push({
      id,
      title: 'llm_test',
      type: TabType.LLM_TEST,
      closable: true,
      data: {},
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
    createLLMTestTab,
    setActiveTab,
    closeTab,
    syncTerminalTabs,
    initialize: syncTerminalTabs,
  }
})
