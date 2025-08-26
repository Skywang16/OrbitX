import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'
import { createMessage } from '@/ui/composables/message-api'
import { TabType } from '@/types'
import { useI18n } from 'vue-i18n'
import { windowApi } from '@/api/window'
import { useAIChatStore } from '@/components/AIChatSidebar'

export class ShortcutActionsService {
  private get tabManagerStore() {
    return useTabManagerStore()
  }

  private get terminalStore() {
    return useTerminalStore()
  }

  private get t() {
    return useI18n().t
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
    const activeTab = this.tabManagerStore.activeTab

    if (!activeTab) {
      return true // 没有活动标签页，阻止默认行为
    }

    // 直接关闭当前标签页，不再限制最后一个终端标签页
    this.tabManagerStore.closeTab(activeTab.id)
    return true
  }

  async newWindow(): Promise<boolean> {
    if ((window as unknown as { __TAURI__?: unknown }).__TAURI__) {
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
    this.tabManagerStore.createSettingsTab()
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

  async increaseOpacity(): Promise<boolean> {
    try {
      const currentOpacity = await windowApi.getWindowOpacity()
      const newOpacity = Math.min(currentOpacity + 0.05, 1.0)
      await windowApi.setWindowOpacity(newOpacity)
      return true
    } catch (error) {
      console.error('增加透明度失败:', error)
      return false
    }
  }

  async decreaseOpacity(): Promise<boolean> {
    try {
      const currentOpacity = await windowApi.getWindowOpacity()
      const newOpacity = Math.max(currentOpacity - 0.05, 0.1)
      await windowApi.setWindowOpacity(newOpacity)
      return true
    } catch (error) {
      console.error('减少透明度失败:', error)
      return false
    }
  }

  toggleAISidebar(): boolean {
    try {
      const aiChatStore = useAIChatStore()
      aiChatStore.toggleSidebar()
      return true
    } catch (error) {
      console.error('切换AI侧边栏失败:', error)
      return false
    }
  }

  async toggleWindowPin(): Promise<boolean> {
    try {
      await windowApi.toggleAlwaysOnTop()
      return true
    } catch (error) {
      console.error('切换窗口钉住状态失败:', error)
      return false
    }
  }
}

export const shortcutActionsService = new ShortcutActionsService()
