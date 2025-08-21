import { useTabManagerStore } from '@/stores/TabManager'
import { useTerminalStore } from '@/stores/Terminal'
import { useSettingsStore } from '@/components/settings/store'

export class ShortcutActionsService {
  private get tabManagerStore() {
    return useTabManagerStore()
  }

  private get terminalStore() {
    return useTerminalStore()
  }

  private get settingsStore() {
    return useSettingsStore()
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

    if (tabs.length <= 1 || !activeTab) {
      return false
    }

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
    this.settingsStore.openSettings()
    return true
  }

  async toggleTheme(): Promise<boolean> {
    const themeManager = this.settingsStore.themeManager
    const currentTheme = themeManager.currentThemeName.value
    const newTheme = currentTheme === 'dark' ? 'light' : 'dark'
    await themeManager.switchToTheme(newTheme)
    return true
  }

  clearTerminal(): boolean {
    const activeTerminal = document.querySelector('.terminal-active .xterm-terminal')
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
