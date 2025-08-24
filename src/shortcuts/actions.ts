import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'
import { createMessage } from '@/ui/composables/message-api'
import { TabType } from '@/types'

export class ShortcutActionsService {
  private get tabManagerStore() {
    return useTabManagerStore()
  }

  private get terminalStore() {
    return useTerminalStore()
  }

  switchToTab(index: number): boolean {
    const tabs = this.tabManagerStore.tabs
    if (index >= 0 && index < tabs.length) {
      this.tabManagerStore.setActiveTab(tabs[index].id)
      return true
    }
    return false
  }

  switchToLastTab(): boolean {
    const tabs = this.tabManagerStore.tabs
    if (tabs.length > 0) {
      this.tabManagerStore.setActiveTab(tabs[tabs.length - 1].id)
      return true
    }
    return false
  }

  async newTab(): Promise<boolean> {
    await this.terminalStore.createTerminal()
    return true
  }

  closeCurrentTab(): boolean {
    const tabs = this.tabManagerStore.tabs
    const activeTab = this.tabManagerStore.activeTab

    if (!activeTab) {
      return true // 没有活动标签页，阻止默认行为
    }

    // 如果当前是终端标签页，检查是否为最后一个终端标签页
    if (activeTab.type === TabType.TERMINAL) {
      const terminalTabs = tabs.filter(tab => tab.type === TabType.TERMINAL)
      if (terminalTabs.length <= 1) {
        createMessage.warning('无法关闭最后一个终端标签页')
        return true // 阻止关闭最后一个终端标签页
      }
    }

    // 对于设置页等其他类型的标签页，可以直接关闭
    this.tabManagerStore.closeTab(activeTab.id)
    return true
  }

  async newWindow(): Promise<boolean> {
    if ((window as any).__TAURI__) {
      return false
    }
    window.open(window.location.href, '_blank')
    return true
  }

  async copyToClipboard(): Promise<boolean> {
    return true
  }

  async pasteFromClipboard(): Promise<boolean> {
    return true
  }

  terminalSearch(): boolean {
    const event = new CustomEvent('open-terminal-search')
    document.dispatchEvent(event)
    return true
  }

  acceptCompletion(): boolean {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('accept-completion', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  openSettings(): boolean {
    this.tabManagerStore.createSettingsTab('theme')
    return true
  }

  clearTerminal(): boolean {
    const activeTerminal = document.querySelector('.terminal-active')
    if (activeTerminal) {
      const event = new CustomEvent('clear-terminal', { bubbles: true })
      activeTerminal.dispatchEvent(event)
      return true
    }
    return false
  }

  increaseFontSize(): boolean {
    document.dispatchEvent(
      new CustomEvent('font-size-change', {
        detail: { action: 'increase' },
      })
    )
    return true
  }

  decreaseFontSize(): boolean {
    document.dispatchEvent(
      new CustomEvent('font-size-change', {
        detail: { action: 'decrease' },
      })
    )
    return true
  }
}

export const shortcutActionsService = new ShortcutActionsService()
