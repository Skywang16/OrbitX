import { useEditorStore } from '@/stores/Editor'

import { windowApi } from '@/api/window'
import { setWindowOpacity, getWindowOpacity } from '@/api/window/opacity'
import { useAIChatStore } from '@/components/AIChatSidebar'
import { useWindowStore } from '@/stores/Window'
import { useGitStore } from '@/stores/git'

export class ShortcutActionsService {
  private get editorStore() {
    return useEditorStore()
  }

  switchToTab = (index: number): boolean => {
    const group = this.editorStore.activeGroup
    if (!group) return false

    const tabs = group.tabs
    if (index >= 0 && index < tabs.length) {
      void this.editorStore.setActiveTab(group.id, tabs[index].id)
      return true
    }
    return false
  }

  switchToLastTab = (): boolean => {
    const group = this.editorStore.activeGroup
    if (!group) return false

    const tabs = group.tabs
    if (tabs.length > 0) {
      void this.editorStore.setActiveTab(group.id, tabs[tabs.length - 1].id)
      return true
    }
    return false
  }

  newTab = async (): Promise<boolean> => {
    await this.editorStore.createTerminalTab({ activate: true })
    return true
  }

  closeCurrentTab = (): boolean => {
    const groupId = this.editorStore.activeGroupId
    const activeTab = this.editorStore.activeTab

    if (!activeTab) {
      return true // 没有活动标签页，阻止默认行为
    }

    // 直接关闭当前标签页，不再限制最后一个终端标签页
    void this.editorStore.closeTab(groupId, activeTab.id)
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
    void this.editorStore.createSettingsTab()
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
