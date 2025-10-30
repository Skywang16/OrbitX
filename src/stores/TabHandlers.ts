import { TabType, type AnyTabItem, type TerminalTabItem, type SettingsTabItem } from '@/types'
import type { TabState, TerminalTabState, SettingsTabState } from '@/types/domain/storage'
import { useTerminalStore } from './Terminal'
import { useSessionStore } from './session'

export interface TabHandler<T extends TabState = TabState, R extends AnyTabItem = AnyTabItem> {
  buildTabItem(tab: T): R
  activate(tabId: number): Promise<void>
  close?(tabId: number): Promise<void>
}

const handlers = new Map<TabState['type'], TabHandler>()

export const getHandler = (type: TabState['type']): TabHandler => {
  const handler = handlers.get(type)
  if (!handler) {
    throw new Error(`Tab handler not registered for type: ${type}`)
  }
  return handler
}

export const defaultCloseTab = async (tabId: number): Promise<void> => {
  const sessionStore = useSessionStore()

  const tabIndex = sessionStore.tabs.findIndex(t => t.id === tabId)
  if (tabIndex === -1) return

  const activeTabId = sessionStore.activeTabId
  const remainingTabs = sessionStore.tabs.filter(t => t.id !== tabId)
  sessionStore.updateTabs(remainingTabs)

  if (activeTabId === tabId && remainingTabs.length > 0) {
    const newIndex = Math.min(Math.max(0, tabIndex - 1), remainingTabs.length - 1)
    const newActiveTab = remainingTabs[newIndex]
    await getHandler(newActiveTab.type).activate(newActiveTab.id)
  } else if (remainingTabs.length === 0) {
    sessionStore.setActiveTab(null)
  }
}

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

handlers.set('terminal', {
  buildTabItem: (tab: TerminalTabState): TerminalTabItem => {
    const terminalStore = useTerminalStore()
    const terminal = terminalStore.terminals.find(t => t.id === tab.id)

    return {
      id: tab.id,
      type: TabType.TERMINAL,
      closable: true,
      data: {
        shell: terminal?.shell || 'shell',
        path: terminal ? getDisplayPath(terminal.cwd) : '~',
      },
    }
  },

  activate: async (tabId: number): Promise<void> => {
    await useTerminalStore().setActiveTerminal(tabId)
  },

  close: async (tabId: number): Promise<void> => {
    await useTerminalStore().closeTerminal(tabId)
    await defaultCloseTab(tabId)
  },
})

handlers.set('settings', {
  buildTabItem: (tab: SettingsTabState): SettingsTabItem => {
    return {
      id: tab.id,
      type: TabType.SETTINGS,
      closable: true,
      data: {
        section: tab.data?.lastSection || 'general',
      },
    }
  },

  activate: async (tabId: number): Promise<void> => {
    useSessionStore().setActiveTab(tabId)
  },
})
