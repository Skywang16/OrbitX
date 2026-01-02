import { onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTerminalStore } from '@/stores/Terminal'
import { useTabManagerStore } from '@/stores/TabManager'
import { shortcutActionsService } from '@/shortcuts/actions'

export function useMenuEvents() {
  const terminalStore = useTerminalStore()
  const tabManagerStore = useTabManagerStore()
  const unlisteners: UnlistenFn[] = []

  // 切换标签页 (direction: -1 上一个, 1 下一个)
  const switchTab = (direction: -1 | 1) => {
    const tabs = tabManagerStore.tabs
    const activeId = tabManagerStore.activeTabId
    if (tabs.length <= 1 || activeId === null) return

    const currentIndex = tabs.findIndex(t => t.id === activeId)
    const nextIndex = (currentIndex + direction + tabs.length) % tabs.length
    tabManagerStore.setActiveTab(tabs[nextIndex].id)
  }

  const menuHandlers: [string, () => void][] = [
    // Shell
    ['menu:new-tab', () => terminalStore.createTerminal()],
    ['menu:close-tab', () => shortcutActionsService.closeCurrentTab()],

    // 编辑
    ['menu:find', () => shortcutActionsService.terminalSearch()],
    ['menu:clear-terminal', () => shortcutActionsService.clearTerminal()],

    // 显示
    ['menu:increase-font-size', () => shortcutActionsService.increaseFontSize()],
    ['menu:decrease-font-size', () => shortcutActionsService.decreaseFontSize()],
    ['menu:increase-opacity', () => shortcutActionsService.increaseOpacity()],
    ['menu:decrease-opacity', () => shortcutActionsService.decreaseOpacity()],
    ['menu:toggle-ai-sidebar', () => shortcutActionsService.toggleAISidebar()],
    ['menu:toggle-git-panel', () => shortcutActionsService.toggleGitPanel()],

    // 窗口
    ['menu:toggle-always-on-top', () => shortcutActionsService.toggleWindowPin()],
    ['menu:prev-tab', () => switchTab(-1)],
    ['menu:next-tab', () => switchTab(1)],

    // 设置
    ['menu:preferences', () => tabManagerStore.createSettingsTab()],
  ]

  onMounted(async () => {
    for (const [event, handler] of menuHandlers) {
      unlisteners.push(await listen(event, handler))
    }
  })

  onUnmounted(() => {
    unlisteners.forEach(fn => fn())
  })
}
