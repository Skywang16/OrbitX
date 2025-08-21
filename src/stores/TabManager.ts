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

        // 智能路径显示逻辑
        const displayPath = getDisplayPath(cwd)

        return {
          id: terminal.id,
          title: '', // 终端标签不再使用title字段
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

  /**
   * 智能路径显示逻辑
   * 根据路径特征返回合适的显示名称
   */
  const getDisplayPath = (cwd: string): string => {
    if (!cwd || cwd === '~') return '~'

    try {
      // 移除末尾的斜杠
      const cleanPath = cwd.replace(/\/$/, '')

      // 跨平台Home目录处理
      const homePatterns = [
        /^\/Users\/[^/]+/, // macOS
        /^\/home\/[^/]+/, // Linux
        /^C:\\Users\\[^\\]+/i, // Windows
      ]

      for (const homePattern of homePatterns) {
        if (homePattern.test(cleanPath)) {
          const homeMatch = cleanPath.match(homePattern)?.[0]
          if (homeMatch && cleanPath === homeMatch) {
            return '~' // 用户home目录
          }
          // home子目录显示相对路径
          const relativePath = cleanPath.replace(homePattern, '~')
          const pathParts = relativePath.split(/[/\\]/).filter(p => p.length > 0)
          if (pathParts.length > 0) {
            const lastPart = pathParts[pathParts.length - 1]
            return lastPart.length > 20 ? lastPart.substring(0, 17) + '...' : lastPart
          }
          return '~'
        }
      }

      // 处理系统重要目录
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

      // 对于其他路径，显示最后一级目录名
      const pathParts = cleanPath.split(/[/\\]/).filter(p => p.length > 0)

      if (pathParts.length === 0) return '/'

      const lastPart = pathParts[pathParts.length - 1]

      // 如果是根目录下的直接子目录，显示带斜杠前缀
      if (pathParts.length === 1 && (cleanPath.startsWith('/') || cleanPath.match(/^[A-Z]:\\/i))) {
        return navigator.platform.toLowerCase().includes('win') ? lastPart : `/${lastPart}`
      }

      // 如果目录名太长，进行截断
      if (lastPart.length > 20) {
        return lastPart.substring(0, 17) + '...'
      }

      return lastPart
    } catch (error) {
      console.warn('路径处理错误:', error, '原始路径:', cwd)
      // 失败时的降级处理
      const parts = cwd.split(/[/\\]/).filter(p => p.length > 0)
      return parts.length > 0 ? parts[parts.length - 1] : '~'
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
