import { invoke } from '@/utils/request'
import type { AppConfig, ConfigFileInfo, Theme, ThemeInfo, ThemeConfigStatus } from './types'

class ThemeAPI {
  getThemeConfigStatus = async (): Promise<ThemeConfigStatus> => {
    return await invoke<ThemeConfigStatus>('theme_get_config_status')
  }

  getCurrentTheme = async (): Promise<Theme> => {
    return await invoke<Theme>('theme_get_current')
  }

  getAvailableThemes = async (): Promise<ThemeInfo[]> => {
    return await invoke<ThemeInfo[]>('theme_get_available')
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

  updateConfig = async (config: AppConfig): Promise<void> => {
    await invoke<void>('config_update', { newConfig: config })
  }

  saveConfig = async (): Promise<void> => {
    await invoke<void>('config_save')
  }

  validateConfig = async (): Promise<void> => {
    await invoke<void>('config_validate')
  }

  resetConfig = async (): Promise<void> => {
    await invoke<void>('reset_config')
  }

  reloadConfig = async (): Promise<void> => {
    await invoke<void>('reload_config')
  }

  getConfigFileInfo = async (): Promise<ConfigFileInfo> => {
    return await invoke<ConfigFileInfo>('config_get_file_info')
  }

  exportConfig = async (): Promise<string> => {
    return await invoke<string>('export_config')
  }

  importConfig = async (configData: string): Promise<void> => {
    await invoke<void>('import_config', { data: configData })
  }

  resetToDefaults = async (): Promise<void> => {
    await invoke<void>('config_reset_to_defaults')
  }

  getFilePath = async (): Promise<string> => {
    return await invoke<string>('config_get_file_path')
  }

  getFileInfo = async (): Promise<ConfigFileInfo> => {
    return await invoke<ConfigFileInfo>('config_get_file_info')
  }

  openFile = async (): Promise<void> => {
    await invoke<void>('config_open_file')
  }

  getConfigFolderPath = async (): Promise<string> => {
    return await invoke<string>('config_get_folder_path')
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

  getAvailableThemes = async (): Promise<ThemeInfo[]> => {
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
