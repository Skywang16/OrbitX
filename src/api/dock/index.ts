import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface TabEntry {
  id: string
  title: string
}

export const dockApi = {
  async updateTabs(tabs: TabEntry[], activeTabId?: string | null): Promise<void> {
    await invoke<void>('dock_update_tabs', { tabs, activeTabId: activeTabId || null })
  },

  async getTabs(): Promise<TabEntry[]> {
    return await invoke<TabEntry[]>('dock_get_tabs')
  },

  async clearTabs(): Promise<void> {
    await invoke<void>('dock_clear_tabs')
  },

  async onDockSwitchTab(callback: (payload: { tabId: number }) => void): Promise<UnlistenFn> {
    return listen<{ tabId: number }>('dock_switch_tab', event => callback(event.payload))
  },
}
