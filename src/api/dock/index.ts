import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface TabEntry {
  id: string
  title: string
}

export const dockApi = {
  updateTabs: async (tabs: TabEntry[], activeTabId?: string | null): Promise<void> => {
    await invoke<void>('dock_update_tabs', { tabs, activeTabId: activeTabId || null })
  },

  getTabs: async (): Promise<TabEntry[]> => {
    return await invoke<TabEntry[]>('dock_get_tabs')
  },

  clearTabs: async (): Promise<void> => {
    await invoke<void>('dock_clear_tabs')
  },

  onDockSwitchTab: async (callback: (payload: { tabId: number }) => void): Promise<UnlistenFn> => {
    return listen<{ tabId: number }>('dock_switch_tab', event => callback(event.payload))
  },
}
