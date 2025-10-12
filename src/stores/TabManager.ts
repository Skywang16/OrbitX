import { defineStore } from 'pinia'
import { computed, watch } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import { TabType, type TabItem } from '@/types'
import { useTerminalStore } from './Terminal'
import { useSessionStore } from './session'
import { dockApi } from '@/api'

/**
 * TabManager - 计算层，不存储数据
 * 唯一数据源：SessionStore.tabs + TerminalStore.terminals
 */
export const useTabManagerStore = defineStore('TabManager', () => {
  const terminalStore = useTerminalStore()
  const sessionStore = useSessionStore()

  /**
   * tabs - 计算属性，从 SessionStore.tabs 和 TerminalStore.terminals 组合得出
   * SessionStore.tabs 提供顺序和基础信息
   * TerminalStore.terminals 提供终端运行时数据
   */
  const tabs = computed<TabItem[]>(() => {
    // 一次性构建 Map，避免重复 find - O(1) 查找
    const terminalMap = new Map(terminalStore.terminals.map(t => [t.id, t]))

    return sessionStore.tabs.map(tab => {
      if (tab.type === 'terminal') {
        const terminal = terminalMap.get(tab.id)
        return {
          id: tab.id,
          type: TabType.TERMINAL,
          closable: true,
          // 运行时数据从 TerminalStore 取
          shell: terminal?.shell,
          path: terminal ? getDisplayPath(terminal.cwd) : '~',
          title: terminal?.title || '',
        }
      } else {
        // settings tab
        return {
          id: tab.id,
          type: TabType.SETTINGS,
          title: 'settings',
          closable: true,
          data: tab.data,
        }
      }
    })
  })

  /**
   * activeTabId - 直接从 SessionStore 读取
   */
  const activeTabId = computed(() => sessionStore.activeTabId ?? null)

  /**
   * activeTab - 当前活跃的 tab
   */
  const activeTab = computed(() => tabs.value.find(tab => tab.id === activeTabId.value))

  /**
   * 监听 TerminalStore 变化，更新 Dock 菜单
   */
  watch(
    () => terminalStore.terminals,
    () => {
      updateDockMenu()
    },
    { deep: true }
  )

  /**
   * getDisplayPath - 路径显示逻辑
   */

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

  /**
   * 创建 Settings tab
   */
  const createSettingsTab = (): string => {
    const existing = sessionStore.tabs.find(tab => tab.type === 'settings')
    if (existing) {
      setActiveTab(existing.id)
      return String(existing.id)
    }

    const id = `settings-${uuidv4()}`
    const newTab = {
      type: 'settings' as const,
      id,
      active: false,
      data: {
        lastSection: 'general',
      },
    }

    // 直接更新 SessionStore
    sessionStore.updateTabs([...sessionStore.tabs, newTab])
    setActiveTab(id)

    return id
  }

  /**
   * 更新 Settings tab 的 section
   */
  const updateSettingsTabSection = (tabId: string, section: string) => {
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

  /**
   * 获取 Settings tab 的 section
   */
  const getSettingsTabSection = (tabId: string): string | undefined => {
    const tab = sessionStore.tabs.find(t => t.type === 'settings' && t.id === tabId)
    if (tab && tab.type === 'settings') {
      return tab.data.lastSection
    }
    return undefined
  }

  /**
   * 设置活跃 tab
   */
  const setActiveTab = (tabId: number | string) => {
    const tab = sessionStore.tabs.find(t => t.id === tabId)
    if (!tab) return

    if (tab.type === 'terminal' && typeof tab.id === 'number') {
      terminalStore.setActiveTerminal(tab.id)
    } else {
      // 非终端 tab，直接更新 SessionStore
      sessionStore.setActiveTabId(tabId)
    }
  }

  /**
   * 关闭 tab
   */
  const closeTab = async (tabId: number | string) => {
    const tabIndex = sessionStore.tabs.findIndex(tab => tab.id === tabId)
    if (tabIndex === -1) return

    const tab = sessionStore.tabs[tabIndex]

    if (tab.type === 'terminal' && typeof tab.id === 'number') {
      // 终端 tab，由 TerminalStore 处理（会自动同步到 SessionStore）
      await terminalStore.closeTerminal(tab.id)
      return
    }

    // 非终端 tab（如 settings），直接从 SessionStore 删除
    const remainingTabs = sessionStore.tabs.filter(t => t.id !== tabId)
    sessionStore.updateTabs(remainingTabs)

    // 处理活跃 tab 切换
    if (activeTabId.value === tabId && remainingTabs.length > 0) {
      const newIndex = Math.max(0, tabIndex - 1)
      setActiveTab(remainingTabs[newIndex].id)
    } else if (remainingTabs.length === 0) {
      sessionStore.setActiveTabId(null)
    }
  }

  /**
   * 关闭左侧所有 tabs
   */
  const closeLeftTabs = async (currentTabId: number | string) => {
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

  /**
   * 关闭右侧所有 tabs
   */
  const closeRightTabs = async (currentTabId: number | string) => {
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

  /**
   * 关闭其他所有 tabs
   */
  const closeOtherTabs = async (currentTabId: number | string) => {
    const idsToClose = tabs.value.filter(tab => tab.id !== currentTabId && tab.closable).map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  /**
   * 关闭所有 tabs
   */
  const closeAllTabs = async () => {
    const idsToClose = tabs.value.filter(tab => tab.closable).map(t => t.id)
    for (const id of idsToClose) {
      await closeTab(id)
    }
  }

  /**
   * 更新 Dock 菜单
   */
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

  /**
   * 初始化 - tabs 和 activeTabId 都是计算属性，从 SessionStore 自动读取
   */
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
    getDisplayPath,
    initialize,
  }
})
