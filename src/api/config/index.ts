import { invoke } from '@/utils/request'
import type { AppConfig, Theme, ThemeConfigStatus } from './types'

class ThemeAPI {
  getThemeConfigStatus = async (): Promise<ThemeConfigStatus> => {
    return await invoke<ThemeConfigStatus>('theme_get_config_status')
  }

  getCurrentTheme = async (): Promise<Theme> => {
    return await invoke<Theme>('theme_get_current')
  }

  getAvailableThemes = async (): Promise<Theme[]> => {
    return await invoke<Theme[]>('theme_get_available')
  }

  setTerminalTheme = async (name: string): Promise<void> => {
    await invoke<void>('theme_set_terminal', { themeName: name })
  }

  setFollowSystemTheme = async (followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> => {
    await invoke<void>('theme_set_follow_system', {
      followSystem,
      lightTheme: lightTheme || null,
      darkTheme: darkTheme || null,
    })
  }
}

export class ConfigApi {
  private themeAPI = new ThemeAPI()

  getConfig = async (): Promise<AppConfig> => {
    return await invoke<AppConfig>('config_get')
  }

  setConfig = async (config: AppConfig): Promise<void> => {
    await invoke<void>('config_set', { newConfig: config })
  }

  resetToDefaults = async (): Promise<void> => {
    await invoke<void>('config_reset_to_defaults')
  }

  openConfigFolder = async (): Promise<void> => {
    await invoke<void>('config_open_folder')
  }

  getThemeConfigStatus = async (): Promise<ThemeConfigStatus> => {
    return this.themeAPI.getThemeConfigStatus()
  }

  getCurrentTheme = async (): Promise<Theme> => {
    return this.themeAPI.getCurrentTheme()
  }

  getAvailableThemes = async (): Promise<Theme[]> => {
    return this.themeAPI.getAvailableThemes()
  }

  setTerminalTheme = async (name: string): Promise<void> => {
    return this.themeAPI.setTerminalTheme(name)
  }

  setFollowSystemTheme = async (followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> => {
    return this.themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
  }

  get theme() {
    return this.themeAPI
  }
}

export const configApi = new ConfigApi()

export const themeAPI = configApi.theme

export type * from './types'
export { ConfigApiError } from './types'

export default configApi
