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

handlers.set('terminal', {
  buildTabItem: (tab: TerminalTabState): TerminalTabItem => {
    const terminalStore = useTerminalStore()
    const terminal = terminalStore.terminals.find(t => t.id === tab.id)

    return {
      id: tab.id,
      type: TabType.TERMINAL,
      closable: true,
      data: {
        shell: terminal ? terminal.shell : tab.data.shell,
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
