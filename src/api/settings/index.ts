import { invoke } from '@/utils/request'
import type { EffectiveSettings, Settings } from './types'

export class SettingsApi {
  getGlobal = async (): Promise<Settings> => {
    return await invoke<Settings>('get_global_settings')
  }

  updateGlobal = async (settings: Settings): Promise<void> => {
    await invoke<void>('update_global_settings', { settings })
  }

  getWorkspace = async (workspace: string): Promise<Settings | null> => {
    return await invoke<Settings | null>('get_workspace_settings', { workspace })
  }

  updateWorkspace = async (workspace: string, settings: Settings): Promise<void> => {
    await invoke<void>('update_workspace_settings', { workspace, settings })
  }

  getEffective = async (workspace?: string): Promise<EffectiveSettings> => {
    return await invoke<EffectiveSettings>('get_effective_settings', { workspace: workspace || null })
  }
}

export const settingsApi = new SettingsApi()
export type * from './types'
export default settingsApi
