import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'

import { windowApi } from '@/api/window'
import { setWindowOpacity, getWindowOpacity } from '@/api/window/opacity'
import { useAIChatStore } from '@/components/AIChatSidebar'
import { useWindowStore } from '@/stores/Window'
import { useGitStore } from '@/stores/git'

export class ShortcutActionsService {
  private get tabManagerStore() {
    return useTabManagerStore()
  }

  private get terminalStore() {
    return useTerminalStore()
  }

  switchToTab = (index: number): boolean => {
    const tabs = this.tabManagerStore.tabs
    if (index >= 0 && index < tabs.length) {
      this.tabManagerStore.setActiveTab(tabs[index].id)
      return true
    }
    return false
  }

  switchToLastTab = (): boolean => {
    const tabs = this.tabManagerStore.tabs
    if (tabs.length > 0) {
      this.tabManagerStore.setActiveTab(tabs[tabs.length - 1].id)
      return true
    }
    return false
  }

  newTab = async (): Promise<boolean> => {
    await this.terminalStore.createTerminal()
    return true
  }

  closeCurrentTab = (): boolean => {
    const activeTab = this.tabManagerStore.activeTab

    if (!activeTab) {
      return true // 没有活动标签页，阻止默认行为
    }

    // 直接关闭当前标签页，不再限制最后一个终端标签页
    this.tabManagerStore.closeTab(activeTab.id)
    return true
  }

  newWindow = async (): Promise<boolean> => {
    if ((window as unknown as { __TAURI__?: unknown }).__TAURI__) {
      return false
    }
    window.open(window.location.href, '_blank')
    return true
  }

  copyToClipboard = async (): Promise<boolean> => {
    return true
  }

  pasteFromClipboard = async (): Promise<boolean> => {
    return true
  }

  terminalSearch = (): boolean => {
    const event = new CustomEvent('open-terminal-search')
    document.dispatchEvent(event)
    return true
  }

  acceptCompletion = (): boolean => {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('accept-completion', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  openSettings = (): boolean => {
    this.tabManagerStore.createSettingsTab()
    return true
  }

  clearTerminal = (): boolean => {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('clear-terminal', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  increaseFontSize = (): boolean => {
    document.dispatchEvent(
      new CustomEvent('font-size-change', {
        detail: { action: 'increase' },
      })
    )
    return true
  }

  decreaseFontSize = (): boolean => {
    document.dispatchEvent(
      new CustomEvent('font-size-change', {
        detail: { action: 'decrease' },
      })
    )
    return true
  }

  increaseOpacity = async (): Promise<boolean> => {
    const currentOpacity = await getWindowOpacity()
    const newOpacity = Math.min(currentOpacity + 0.05, 1.0)
    await setWindowOpacity(newOpacity)
    return true
  }

  decreaseOpacity = async (): Promise<boolean> => {
    const currentOpacity = await getWindowOpacity()
    const newOpacity = Math.max(currentOpacity - 0.05, 0.05)
    await setWindowOpacity(newOpacity)
    return true
  }

  toggleAISidebar = (): boolean => {
    const aiChatStore = useAIChatStore()
    aiChatStore.toggleSidebar()
    return true
  }

  toggleGitPanel = (): boolean => {
    const gitStore = useGitStore()
    gitStore.togglePanel()
    return true
  }

  toggleWindowPin = async (): Promise<boolean> => {
    const newState = await windowApi.toggleAlwaysOnTop()
    const windowStore = useWindowStore()
    windowStore.setAlwaysOnTop(newState)
    return true
  }
}

export const shortcutActionsService = new ShortcutActionsService()
